use std::path::Path;
use std::process::Command;

use log::{debug, info, warn};
use serde_extensions::Overwrite;

use crate::{
    lssg_error::{LssgError, LssgErrorKind},
    sitetree::{Resource, SiteNodeKind, SiteTree},
    tree::DFS,
};

use super::RendererModule;

#[derive(Debug, Clone, Overwrite)]
pub struct MediaOptions {
    /// Enable image optimization
    pub optimize_images: bool,
    /// Image quality (1-100)
    pub image_quality: u8,
    /// Enable video optimization  
    pub optimize_videos: bool,
    /// Video quality CRF (0-51, lower = better quality)
    pub video_crf: u8,
    /// Convert images to WebP
    pub convert_to_webp: bool,
    /// Maximum image width
    pub max_width: Option<u32>,
    /// Maximum image height
    pub max_height: Option<u32>,
    /// Minimum file size (in bytes) to trigger resizing
    pub resize_threshold_bytes: usize,
    /// WebP quality (1-100, 95+ uses lossless)
    pub webp_quality: u8,
    /// Enable FFmpeg for video optimization
    pub use_ffmpeg: bool,
}

impl Default for MediaOptions {
    fn default() -> Self {
        Self {
            optimize_images: true,
            image_quality: 85,
            optimize_videos: true,
            video_crf: 25,
            convert_to_webp: true,
            max_width: Some(1920),
            max_height: Some(1080),
            resize_threshold_bytes: 1000_000,
            webp_quality: 95,
            use_ffmpeg: true,
        }
    }
}

pub struct MediaModule {
    options: MediaOptions,
}

impl MediaModule {
    pub fn new() -> Self {
        Self {
            options: MediaOptions::default(),
        }
    }

    fn optimize_image(
        &self,
        resource: &mut Resource,
        original_name: &str,
    ) -> Result<Option<String>, LssgError> {
        info!("Starting optimization for image: {}", original_name);

        let data = resource.data()?;
        info!("Loaded image data: {} bytes", data.len());

        let img = image::load_from_memory(&data).map_err(|e| {
            LssgError::new(format!("Failed to load image: {}", e), LssgErrorKind::Io)
        })?;

        let original_width = img.width();
        let original_height = img.height();
        info!(
            "Successfully loaded image: {}x{}",
            original_width, original_height
        );

        let mut optimized_img = img;

        // Only resize if the image is both too large in dimensions AND has a significant file size
        // This prevents unnecessary quality loss on small images
        let should_resize =
            if let (Some(max_w), Some(max_h)) = (self.options.max_width, self.options.max_height) {
                let dimensions_too_large =
                    optimized_img.width() > max_w || optimized_img.height() > max_h;
                let file_size_significant = data.len() > self.options.resize_threshold_bytes;
                dimensions_too_large && file_size_significant
            } else {
                false
            };

        if should_resize {
            let (max_w, max_h) = (
                self.options.max_width.unwrap(),
                self.options.max_height.unwrap(),
            );
            optimized_img =
                optimized_img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
            info!(
                "Resized image {} from {}x{} to {}x{} (file size: {} bytes)",
                original_name,
                original_width,
                original_height,
                optimized_img.width(),
                optimized_img.height(),
                data.len()
            );
        } else if let (Some(max_w), Some(max_h)) = (self.options.max_width, self.options.max_height)
        {
            if optimized_img.width() > max_w || optimized_img.height() > max_h {
                info!(
                    "Skipping resize for {} ({}x{}, {} bytes) - file size too small to warrant quality loss",
                    original_name,
                    optimized_img.width(),
                    optimized_img.height(),
                    data.len()
                );
            }
        }

        let mut buffer = Vec::new();
        let new_extension = if self.options.convert_to_webp {
            // Convert to WebP - convert the image to RGBA8 for webp crate compatibility
            let rgba_img = optimized_img.to_rgba8();
            let (width, height) = rgba_img.dimensions();

            let encoder = webp::Encoder::from_rgba(&rgba_img, width, height);
            let webp_data = encoder.encode(self.options.image_quality as f32).to_vec();
            buffer = webp_data;
            Some("webp")
        } else {
            // Use original format with optimization
            use image::ImageFormat;
            let format = match Path::new(original_name)
                .extension()
                .and_then(|s| s.to_str())
            {
                Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
                Some("png") => ImageFormat::Png,
                _ => ImageFormat::Jpeg,
            };

            optimized_img
                .write_to(&mut std::io::Cursor::new(&mut buffer), format)
                .map_err(|e| {
                    LssgError::new(format!("Failed to encode image: {}", e), LssgErrorKind::Io)
                })?;
            None
        };

        // Calculate compression ratio
        let compression_ratio = if data.len() > 0 {
            ((data.len() as f64 - buffer.len() as f64) / data.len() as f64) * 100.0
        } else {
            0.0
        };

        // Update resource with optimized data
        *resource = Resource::Static { content: buffer };

        let format_info = if let Some(ext) = new_extension {
            format!("Converted {} to {}", original_name, ext)
        } else {
            format!("Optimized image: {}", original_name)
        };

        info!("{} ({:.1}% size reduction)", format_info, compression_ratio);

        Ok(new_extension.map(|ext| ext.to_string())) // Return new extension if converted
    }

    fn optimize_video(
        &self,
        resource: &mut Resource,
        original_name: &str,
    ) -> Result<(), LssgError> {
        if !self.options.use_ffmpeg {
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
        let input_path = temp_dir.join(format!("lssg_input_{}", original_name));
        let output_path = temp_dir.join(format!("lssg_output_{}", original_name));

        // Write input to temp file
        std::fs::write(&input_path, &data).map_err(|e| {
            LssgError::new(
                format!("Failed to write temp file: {}", e),
                LssgErrorKind::Io,
            )
        })?;

        // Run ffmpeg optimization with simpler, more robust settings
        let crf_str = self.options.video_crf.to_string();
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
        let vf_string = if let (Some(max_w), Some(max_h)) =
            (self.options.max_width, self.options.max_height)
        {
            // Simple scale filter that maintains aspect ratio and ensures even dimensions
            format!("scale='min({},iw)':'min({}*ih/iw,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2", max_w, max_h)
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
        let output = Command::new("ffmpeg").args(&args).output().map_err(|e| {
            LssgError::new(format!("Failed to run ffmpeg: {}", e), LssgErrorKind::Io)
        })?;

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
        let compression_ratio = if data.len() > 0 {
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

    fn is_image_file(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.ends_with(".jpg")
            || name_lower.ends_with(".jpeg")
            || name_lower.ends_with(".png")
            || name_lower.ends_with(".gif")
            || name_lower.ends_with(".bmp")
            || name_lower.ends_with(".webp")
            || name_lower.ends_with(".tiff")
            || name_lower.ends_with(".tif")
    }

    fn is_video_file(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.ends_with(".mp4")
            || name_lower.ends_with(".webm")
            || name_lower.ends_with(".ogg")
            || name_lower.ends_with(".ogv")
            || name_lower.ends_with(".mov")
            || name_lower.ends_with(".avi")
            || name_lower.ends_with(".mkv")
            || name_lower.ends_with(".m4v")
    }
}

impl RendererModule for MediaModule {
    fn id(&self) -> &'static str {
        "media"
    }

    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError> {
        // Get global options from root page if available
        if let SiteNodeKind::Page(page) = &site_tree[site_tree.root()].kind {
            self.options = self.options(page);
        }

        if !self.options.optimize_images && !self.options.optimize_videos {
            info!("Media optimization disabled");
            return Ok(());
        }

        info!("Starting media optimization...");

        // Find all resource nodes
        let resource_ids: Vec<usize> = DFS::new(site_tree)
            .filter(|id| matches!(site_tree[*id].kind, SiteNodeKind::Resource(_)))
            .collect();

        let mut processed_count = 0;
        let mut optimized_count = 0;

        for id in resource_ids {
            let node_name = site_tree[id].name.clone();

            if let SiteNodeKind::Resource(ref mut resource) = &mut site_tree[id].kind {
                let mut optimized = false;

                if self.options.optimize_images && Self::is_image_file(&node_name) {
                    match self.optimize_image(resource, &node_name) {
                        Ok(new_extension) => {
                            optimized = true;
                            // Update filename if converted to WebP
                            if let Some(ext) = new_extension {
                                let new_name = if let Some(dot_pos) = node_name.rfind('.') {
                                    format!("{}.{}", &node_name[..dot_pos], ext)
                                } else {
                                    format!("{}.{}", node_name, ext)
                                };
                                site_tree[id].name = new_name.clone();
                                info!("Updated filename from {} to {}", node_name, new_name);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to optimize image {}: {}", node_name, e);
                        }
                    }
                } else if self.options.optimize_videos && Self::is_video_file(&node_name) {
                    match self.optimize_video(resource, &node_name) {
                        Ok(()) => {
                            optimized = true;
                        }
                        Err(e) => {
                            warn!("Failed to optimize video {}: {}", node_name, e);
                        }
                    }
                }

                if optimized {
                    optimized_count += 1;
                }
                if Self::is_image_file(&node_name) || Self::is_video_file(&node_name) {
                    processed_count += 1;
                }
            }
        }

        info!(
            "Media optimization complete: {}/{} files optimized",
            optimized_count, processed_count
        );
        Ok(())
    }
}
