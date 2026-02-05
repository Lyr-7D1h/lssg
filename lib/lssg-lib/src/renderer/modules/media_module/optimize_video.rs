use std::process::Command;

use log::{debug, info, warn};

use crate::{
    lssg_error::{LssgError, LssgErrorKind},
    sitetree::Resource,
};

use super::MediaOptions;

pub fn optimize_video(
    options: &MediaOptions,
    resource: &mut Resource,
    original_name: &str,
) -> Result<(), LssgError> {
    if !options.use_ffmpeg {
        info!("Video optimization disabled, skipping {}", original_name);
        return Ok(());
    }

    // Check if ffmpeg is available
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        warn!(
            "FFmpeg not found, skipping video optimization for {}",
            original_name
        );
        return Ok(());
    }

    let data = resource.data()?;

    let temp_dir = std::env::temp_dir();
    let input_path = temp_dir.join(format!("lssg_input_{original_name}"));
    let output_path = temp_dir.join(format!("lssg_output_{original_name}"));

    // Write input to temp file
    std::fs::write(&input_path, &data).map_err(|e| {
        LssgError::new(
            format!("Failed to write temp file: {}", e),
            LssgErrorKind::Io,
        )
    })?;

    // Run ffmpeg optimization with simpler, more robust settings
    let crf_str = options.video_crf.to_string();
    let mut args = vec![
        "-i",
        input_path.to_str().unwrap(),
        "-c:v",
        "libx264",
        "-crf",
        &crf_str,
        "-preset",
        "medium",
    ];

    // Prepare video filter string
    let vf_string = if let (Some(max_w), Some(max_h)) = (options.max_width, options.max_height) {
        // Simple scale filter that maintains aspect ratio and ensures even dimensions
        format!(
            "scale='min({},iw)':'min({}*ih/iw,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2",
            max_w, max_h
        )
    } else {
        // Just ensure dimensions are even
        "scale=trunc(iw/2)*2:trunc(ih/2)*2".to_string()
    };

    // Add video filter
    args.extend_from_slice(&["-vf", &vf_string]);

    // Handle audio more carefully - copy if present, skip if not
    args.extend_from_slice(&[
        "-c:a",
        "copy", // Try to copy audio first
        "-avoid_negative_ts",
        "make_zero",
        "-movflags",
        "+faststart", // Optimize for web streaming
        "-y",         // Overwrite output file
        output_path.to_str().unwrap(),
    ]);

    info!("FFmpeg command: ffmpeg {}", args.join(" "));
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .map_err(|e| LssgError::new(format!("Failed to run ffmpeg: {}", e), LssgErrorKind::Io))?;

    // If copying audio failed, try without audio or with AAC encoding
    if !output.status.success() {
        debug!("First attempt failed, trying alternative audio handling...");

        // Try with AAC audio encoding instead of copy
        let mut args_retry = vec![
            "-i",
            input_path.to_str().unwrap(),
            "-c:v",
            "libx264",
            "-crf",
            &crf_str,
            "-preset",
            "medium",
        ];

        // Add the same video filter
        args_retry.extend_from_slice(&["-vf", &vf_string]);

        // Try with no audio processing
        args_retry.extend_from_slice(&[
            "-an", // No audio
            "-avoid_negative_ts",
            "make_zero",
            "-movflags",
            "+faststart",
            "-y",
            output_path.to_str().unwrap(),
        ]);

        debug!("FFmpeg retry command: ffmpeg {}", args_retry.join(" "));
        let retry_output = Command::new("ffmpeg")
            .args(&args_retry)
            .output()
            .map_err(|e| {
                LssgError::new(
                    format!("Failed to run ffmpeg retry: {}", e),
                    LssgErrorKind::Io,
                )
            })?;

        if !retry_output.status.success() {
            let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
            let retry_stdout = String::from_utf8_lossy(&retry_output.stdout);

            debug!(
                "FFmpeg stderr: {}",
                retry_stderr
                    .lines()
                    .take(10)
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
            debug!(
                "FFmpeg stdout: {}",
                retry_stdout.lines().take(5).collect::<Vec<_>>().join(" | ")
            );
            debug!("Exit code: {:?}", retry_output.status.code());

            // Try one more time with even simpler settings for WebM files
            if original_name.to_lowercase().ends_with(".webm") {
                debug!(
                    "Attempting WebM-specific optimization for {}",
                    original_name
                );

                let webm_args = vec![
                    "-i",
                    input_path.to_str().unwrap(),
                    "-c:v",
                    "libvpx-vp9", // Use VP9 for WebM
                    "-crf",
                    "30", // Slightly lower quality for compatibility
                    "-b:v",
                    "0",   // Use CRF mode
                    "-an", // No audio to avoid codec issues
                    "-f",
                    "webm", // Force WebM format
                    "-y",
                    output_path.to_str().unwrap(),
                ];

                info!("FFmpeg WebM command: ffmpeg {}", webm_args.join(" "));
                let webm_output =
                    Command::new("ffmpeg")
                        .args(&webm_args)
                        .output()
                        .map_err(|e| {
                            LssgError::new(
                                format!("Failed to run ffmpeg WebM: {}", e),
                                LssgErrorKind::Io,
                            )
                        })?;

                if !webm_output.status.success() {
                    let webm_stderr = String::from_utf8_lossy(&webm_output.stderr);
                    warn!(
                        "WebM optimization also failed: {}",
                        webm_stderr.lines().take(5).collect::<Vec<_>>().join(" | ")
                    );
                    // Cleanup and don't fail the build
                    let _ = std::fs::remove_file(&input_path);
                    let _ = std::fs::remove_file(&output_path);
                    return Ok(());
                } else {
                    info!("WebM optimization succeeded for {}", original_name);
                    // Continue to read the optimized file below
                }
            } else {
                warn!(
                    "FFmpeg stderr: {}",
                    retry_stderr
                        .lines()
                        .take(10)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                warn!(
                    "FFmpeg stdout: {}",
                    retry_stdout.lines().take(5).collect::<Vec<_>>().join(" | ")
                );
                warn!("Exit code: {:?}", retry_output.status.code());
                let _ = std::fs::remove_file(&input_path);
                let _ = std::fs::remove_file(&output_path);
                return Ok(());
            }
        }
    }

    // Check if output file was created
    if !output_path.exists() {
        warn!(
            "FFmpeg succeeded but output file {} was not created",
            output_path.display()
        );
        let _ = std::fs::remove_file(&input_path);
        return Ok(());
    }

    // Read optimized video
    let optimized_data = std::fs::read(&output_path).map_err(|e| {
        LssgError::new(
            format!("Failed to read optimized video: {}", e),
            LssgErrorKind::Io,
        )
    })?;

    // Calculate compression ratio
    let compression_ratio = if !data.is_empty() {
        ((data.len() as f64 - optimized_data.len() as f64) / data.len() as f64) * 100.0
    } else {
        0.0
    };

    // Update resource with optimized data
    *resource = Resource::Static {
        content: optimized_data,
    };

    // Cleanup temp files
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    info!(
        "Optimized video: {} ({:.1}% size reduction)",
        original_name, compression_ratio
    );
    Ok(())
}
