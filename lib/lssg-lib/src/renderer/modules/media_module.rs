mod optimize_image;
mod optimize_video;

pub use optimize_image::optimize_image;
pub use optimize_video::optimize_video;

use log::info;
use serde_extensions::Overwrite;

use crate::{
    lssg_error::LssgError,
    sitetree::{SiteId, SiteNodeKind},
    tree::Dfs,
};

use super::RendererModule;
use crate::renderer::InitContext;

#[derive(Debug, Clone, Overwrite)]
pub struct MediaOptions {
    /// Enable media optimization
    pub enabled: bool,

    /// Enable image optimization
    pub optimize_images: bool,
    /// Image quality (1-100)
    pub image_quality: u8,
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

    /// Enable video optimization  
    pub optimize_videos: bool,
    /// Enable FFmpeg for video optimization
    pub use_ffmpeg: bool,
    /// Video quality CRF (0-51, lower = better quality)
    pub video_crf: u8,
}

impl Default for MediaOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            optimize_images: true,
            image_quality: 85,
            optimize_videos: true,
            video_crf: 25,
            convert_to_webp: true,
            max_width: Some(1920),
            max_height: Some(1080),
            resize_threshold_bytes: 1_000_000,
            webp_quality: 95,
            use_ffmpeg: true,
        }
    }
}

#[derive(Default)]
pub struct MediaModule {}

impl RendererModule for MediaModule {
    fn id(&self) -> &'static str {
        "media"
    }

    fn init(&mut self, InitContext { site_tree, .. }: InitContext) -> Result<(), LssgError> {
        info!("Starting media optimization...");

        // Find all resource nodes
        let resource_ids: Vec<SiteId> = Dfs::new(site_tree)
            .filter(|id| matches!(site_tree[*id].kind, SiteNodeKind::Resource(_)))
            .collect();

        let mut processed_count = 0;
        let mut optimized_count = 0;

        for id in resource_ids {
            let options: MediaOptions = self.propegated_options(id, site_tree);

            if !options.enabled {
                log::debug!("Media disabled on {id}");
                continue;
            }

            let node_name = site_tree[id].name().clone();
            if let SiteNodeKind::Resource(resource) = &mut site_tree[id].kind {
                let mut optimized = false;

                if options.optimize_images && is_image_file(&node_name) {
                    match optimize_image(&options, resource, &node_name) {
                        Ok(new_extension) => {
                            optimized = true;
                            // Update filename if converted to WebP
                            if let Some(ext) = new_extension {
                                let new_name = if let Some(dot_pos) = node_name.rfind('.') {
                                    format!("{}.{}", &node_name[..dot_pos], ext)
                                } else {
                                    format!("{}.{}", node_name, ext)
                                };
                                site_tree[id].set_name(&new_name);
                                info!("Updated filename from {} to {}", node_name, new_name);
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to optimize image {}: {}", node_name, e);
                        }
                    }
                } else if options.optimize_videos && is_video_file(&node_name) {
                    match optimize_video(&options, resource, &node_name) {
                        Ok(()) => {
                            optimized = true;
                        }
                        Err(e) => {
                            log::warn!("Failed to optimize video {}: {}", node_name, e);
                        }
                    }
                }

                if optimized {
                    optimized_count += 1;
                }
                if is_image_file(&node_name) || is_video_file(&node_name) {
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

pub fn is_image_file(name: &str) -> bool {
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

pub fn is_video_file(name: &str) -> bool {
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
