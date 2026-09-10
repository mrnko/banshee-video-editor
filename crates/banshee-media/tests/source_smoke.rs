use banshee_domain::{
    default_presets, AnalysisSettings, CropMode, CropSettings, EditDecisionList, TimeRange,
};
use banshee_media::MediaTools;
use std::{collections::HashMap, path::PathBuf};

#[tokio::test]
#[ignore = "requires BANSHEE_SMOKE_VIDEO and local FFmpeg"]
async fn analyzes_and_renders_real_source() {
    let source_path =
        PathBuf::from(std::env::var("BANSHEE_SMOKE_VIDEO").expect("BANSHEE_SMOKE_VIDEO"));
    let tools = MediaTools::discover().unwrap();
    let source = tools.probe(&source_path).await.unwrap();
    assert!(source.duration_seconds > 1.0);
    let root = std::env::temp_dir().join("banshee-source-smoke");
    let mut settings = AnalysisSettings::default();
    settings.moment_count = Some(1);
    let clips = tools
        .analyze_local("smoke", &source, &settings, &root.join("thumbnails"))
        .await
        .unwrap();
    assert_eq!(clips.len(), 1);
    let clip = &clips[0];
    let edit = EditDecisionList {
        project_id: "smoke".into(),
        candidate_id: clip.id.clone(),
        trim_start: clip.start_seconds,
        trim_end: (clip.start_seconds + 3.0).min(clip.end_seconds),
        removed_ranges: vec![TimeRange {
            start: clip.start_seconds + 1.0,
            end: clip.start_seconds + 1.5,
        }],
        crop: CropSettings {
            mode: CropMode::Center,
            zoom: 1.0,
            center_x: 0.5,
            center_y: 0.5,
        },
        overlays: vec![],
        revision: 1,
        cut_points: vec![],
        fps: None,
        track_visibility: Default::default(),
    };
    let output = root.join("smoke.mp4");
    tools
        .render(
            &source,
            &edit,
            &default_presets()[0],
            &HashMap::new(),
            &output,
        )
        .await
        .unwrap();
    assert!(output.metadata().unwrap().len() > 10_000);

    let mut blurred_edit = edit.clone();
    blurred_edit.trim_end = (blurred_edit.trim_start + 2.0).min(clip.end_seconds);
    blurred_edit.removed_ranges.clear();
    blurred_edit.crop.mode = CropMode::BlurredBackground;
    let blurred_output = root.join("smoke-blurred.mp4");
    tools
        .render(
            &source,
            &blurred_edit,
            &default_presets()[0],
            &HashMap::new(),
            &blurred_output,
        )
        .await
        .unwrap();
    assert!(blurred_output.metadata().unwrap().len() > 10_000);

    let mut upscale_edit = edit.clone();
    upscale_edit.trim_end = (upscale_edit.trim_start + 1.0).min(clip.end_seconds);
    upscale_edit.removed_ranges.clear();
    let input_frames = root.join("upscale-input");
    tools
        .extract_vertical_frames(&source, &upscale_edit, 5, &input_frames)
        .await
        .unwrap();
    let intermediate = root.join("upscale-lossless.mkv");
    let prepared = tools
        .assemble_upscaled_clip(
            &source,
            upscale_edit.trim_start,
            upscale_edit.trim_end - upscale_edit.trim_start,
            5,
            &input_frames,
            &intermediate,
        )
        .await
        .unwrap();
    assert!(prepared.height > prepared.width);
}
