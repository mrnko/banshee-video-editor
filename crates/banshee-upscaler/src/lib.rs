use anyhow::{bail, Context, Result};
use banshee_domain::UpscaleMode;
use std::{
    env,
    path::{Path, PathBuf},
};
use tokio::process::Command;

fn background_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    command
}

#[derive(Debug, Clone)]
pub struct Upscaler {
    executable: Option<PathBuf>,
}

impl Upscaler {
    pub fn discover() -> Self {
        let executable = env::var_os("REALESRGAN_PATH")
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| {
                let path = PathBuf::from("tools/bin/realesrgan-ncnn-vulkan.exe");
                path.is_file().then_some(path)
            });
        Self { executable }
    }

    pub fn available(&self) -> bool {
        self.executable.is_some()
    }

    pub async fn enhance_frames(
        &self,
        input: &Path,
        output: &Path,
        mode: &UpscaleMode,
        scale: u8,
    ) -> Result<()> {
        let executable = self
            .executable
            .as_ref()
            .context("Real-ESRGAN не встановлено")?;
        let model = match mode {
            UpscaleMode::Anime => "realesr-animevideov3",
            UpscaleMode::General => "realesrgan-x4plus",
            UpscaleMode::Auto => "realesr-animevideov3",
            UpscaleMode::Off => bail!("AI-upscale вимкнено"),
        };
        std::fs::create_dir_all(output)?;
        let status = background_command(executable)
            .args(["-i"])
            .arg(input)
            .args(["-o"])
            .arg(output)
            .args([
                "-n",
                model,
                "-s",
                &scale.clamp(2, 4).to_string(),
                "-f",
                "png",
            ])
            .status()
            .await?;
        if !status.success() {
            bail!("Real-ESRGAN завершився з помилкою {status}")
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_is_a_clean_state() {
        let upscaler = Upscaler { executable: None };
        assert!(!upscaler.available());
    }
}
