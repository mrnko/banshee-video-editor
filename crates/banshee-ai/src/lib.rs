use anyhow::{anyhow, Context, Result};
use banshee_domain::{CandidateClip, UsageEvent};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashSet, fs};
use uuid::Uuid;

pub const RECOMMENDED_MODELS: &[&str] = &[
    "gpt-6-astra",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOption {
    pub id: String,
    pub available: bool,
    pub experimental: bool,
    pub recommended: bool,
}

#[derive(Debug, Clone)]
pub struct ModelPrice {
    pub input_per_million: f64,
    pub cached_input_per_million: f64,
    pub output_per_million: f64,
}

pub fn price_for(model: &str) -> Option<ModelPrice> {
    match model {
        "gpt-6-astra" => Some(ModelPrice {
            input_per_million: 10.0,
            cached_input_per_million: 1.0,
            output_per_million: 50.0,
        }),
        "gpt-5.6-sol" | "gpt-5.6" => Some(ModelPrice {
            input_per_million: 4.0,
            cached_input_per_million: 0.4,
            output_per_million: 20.0,
        }),
        "gpt-5.6-terra" => Some(ModelPrice {
            input_per_million: 2.0,
            cached_input_per_million: 0.2,
            output_per_million: 12.0,
        }),
        "gpt-5.6-luna" => Some(ModelPrice {
            input_per_million: 0.2,
            cached_input_per_million: 0.02,
            output_per_million: 1.2,
        }),
        _ => None,
    }
}

pub fn estimate_cost(
    model: &str,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
) -> f64 {
    let Some(price) = price_for(model) else {
        return 0.0;
    };
    let non_cached = input_tokens.saturating_sub(cached_input_tokens);
    (non_cached as f64 * price.input_per_million
        + cached_input_tokens as f64 * price.cached_input_per_million
        + output_tokens as f64 * price.output_per_million)
        / 1_000_000.0
}

pub struct OpenAiClient {
    http: Client,
    api_key: String,
}

impl OpenAiClient {
    pub fn new(api_key: String) -> Result<Self> {
        let http = Client::builder()
            .user_agent(concat!("Banshee-Video-Editor/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, api_key })
    }

    pub async fn list_models(&self) -> Result<Vec<ModelOption>> {
        let response = self
            .http
            .get("https://api.openai.com/v1/models")
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?;
        let value: Value = response.json().await?;
        let available: HashSet<String> = value["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["id"].as_str().map(str::to_owned))
            .collect();
        let mut models: Vec<ModelOption> = RECOMMENDED_MODELS
            .iter()
            .map(|id| ModelOption {
                id: (*id).into(),
                available: available.contains(*id),
                experimental: false,
                recommended: *id == "gpt-5.6-terra",
            })
            .collect();
        let mut experimental: Vec<_> = available
            .into_iter()
            .filter(|id| id.starts_with("gpt-") && !RECOMMENDED_MODELS.contains(&id.as_str()))
            .map(|id| ModelOption {
                id,
                available: true,
                experimental: true,
                recommended: false,
            })
            .collect();
        experimental.sort_by(|a, b| a.id.cmp(&b.id));
        models.extend(experimental);
        Ok(models)
    }

    pub async fn rank_candidates(
        &self,
        project_id: &str,
        model: &str,
        prompt: &str,
        candidates: &[CandidateClip],
        transcript: Option<&str>,
    ) -> Result<(Vec<AiRanking>, UsageEvent)> {
        let mut content = vec![json!({
            "type": "input_text",
            "text": format!("{prompt}\nОціни {} кандидатів. Збережи їхні id та не змінюй таймкоди.", candidates.len())
        })];
        if let Some(transcript) = transcript.filter(|value| !value.trim().is_empty()) {
            let excerpt: String = transcript.chars().take(20_000).collect();
            content.push(json!({ "type": "input_text", "text": format!("Транскрипція джерела:\n{excerpt}") }));
        }
        for candidate in candidates {
            content.push(json!({
                "type": "input_text",
                "text": format!("Кандидат {}: {:.3}–{:.3} с, локальна оцінка {:.1}.", candidate.id, candidate.start_seconds, candidate.end_seconds, candidate.score)
            }));
            if let Some(path) = &candidate.thumbnail_path {
                if let Ok(bytes) = fs::read(path) {
                    content.push(json!({
                        "type": "input_image",
                        "detail": "low",
                        "image_url": format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))
                    }));
                }
            }
        }
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["rankings"],
            "properties": {
                "rankings": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["id", "score", "reason"],
                        "properties": {
                            "id": { "type": "string" },
                            "score": { "type": "number", "minimum": 0, "maximum": 100 },
                            "reason": { "type": "string" }
                        }
                    }
                }
            }
        });
        let body = json!({
            "model": model,
            "store": false,
            "input": [{ "role": "user", "content": content }],
            "text": { "format": { "type": "json_schema", "name": "highlight_rankings", "strict": true, "schema": schema } }
        });
        let response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let value: Value = response
            .json()
            .await
            .context("OpenAI повернув неочікувану відповідь")?;
        if !status.is_success() {
            return Err(anyhow!(
                "OpenAI API: {}",
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("невідома помилка")
            ));
        }
        let text = extract_output_text(&value).context("OpenAI не повернув structured output")?;
        let parsed: RankingEnvelope = serde_json::from_str(&text)?;
        let input_tokens = value["usage"]["input_tokens"].as_u64().unwrap_or_default();
        let cached_input_tokens = value["usage"]["input_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or_default();
        let output_tokens = value["usage"]["output_tokens"].as_u64().unwrap_or_default();
        let cost = estimate_cost(model, input_tokens, cached_input_tokens, output_tokens);
        let usage = UsageEvent {
            id: Uuid::new_v4().to_string(),
            project_id: Some(project_id.into()),
            candidate_id: None,
            model: model.into(),
            input_tokens,
            cached_input_tokens,
            output_tokens,
            audio_seconds: 0.0,
            estimated_cost_usd: cost,
            created_at: Utc::now(),
        };
        Ok((parsed.rankings, usage))
    }

    pub async fn transcribe(
        &self,
        project_id: &str,
        audio_path: &std::path::Path,
        duration_seconds: f64,
    ) -> Result<(String, UsageEvent)> {
        let bytes = fs::read(audio_path)?;
        let file_name = audio_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("analysis.m4a")
            .to_string();
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(file_name)
            .mime_str("audio/mp4")?;
        let form = reqwest::multipart::Form::new()
            .text("model", "gpt-transcribe")
            .part("file", part);
        let response = self
            .http
            .post("https://api.openai.com/v1/audio/transcriptions")
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;
        let status = response.status();
        let value: Value = response.json().await?;
        if !status.is_success() {
            return Err(anyhow!(
                "OpenAI transcription: {}",
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("невідома помилка")
            ));
        }
        let text = value["text"].as_str().unwrap_or_default().to_string();
        let usage = UsageEvent {
            id: Uuid::new_v4().to_string(),
            project_id: Some(project_id.into()),
            candidate_id: None,
            model: "gpt-transcribe".into(),
            input_tokens: 0,
            cached_input_tokens: 0,
            output_tokens: 0,
            audio_seconds: duration_seconds,
            estimated_cost_usd: duration_seconds / 60.0 * 0.0045,
            created_at: Utc::now(),
        };
        Ok((text, usage))
    }
}

pub async fn fetch_organization_cost(admin_key: &str, start_time: i64) -> Result<f64> {
    let client = Client::builder()
        .user_agent(concat!("Banshee-Video-Editor/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let value: Value = client
        .get("https://api.openai.com/v1/organization/costs")
        .bearer_auth(admin_key)
        .query(&[
            ("start_time", start_time.to_string()),
            ("bucket_width", "1d".into()),
            ("limit", "31".into()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|bucket| bucket["results"].as_array().into_iter().flatten())
        .filter_map(|result| result["amount"]["value"].as_f64())
        .sum())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRanking {
    pub id: String,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct RankingEnvelope {
    rankings: Vec<AiRanking>,
}

fn extract_output_text(value: &Value) -> Option<String> {
    value["output"]
        .as_array()?
        .iter()
        .flat_map(|item| item["content"].as_array().into_iter().flatten())
        .find_map(|content| content["text"].as_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_known_model_cost() {
        let cost = estimate_cost("gpt-5.6-terra", 1_000_000, 0, 1_000_000);
        assert!((cost - 14.0).abs() < 0.0001);
    }

    #[test]
    fn cached_tokens_get_cached_rate() {
        let cost = estimate_cost("gpt-5.6-luna", 1_000_000, 1_000_000, 0);
        assert!((cost - 0.02).abs() < 0.0001);
    }
}
