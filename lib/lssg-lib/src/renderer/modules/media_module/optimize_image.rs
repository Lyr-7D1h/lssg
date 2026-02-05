use std::path::Path;

use log::info;

use crate::{
    lssg_error::{LssgError, LssgErrorKind},
    sitetree::Resource,
};

use super::MediaOptions;

pub fn optimize_image(
    options: &MediaOptions,
    resource: &mut Resource,
    original_name: &str,
) -> Result<Option<String>, LssgError> {
    info!("Starting optimization for image: {}", original_name);

    let data = resource.data()?;
    info!("Loaded image data: {} bytes", data.len());

    let img = image::load_from_memory(&data)
        .map_err(|e| LssgError::new(format!("Failed to load image: {}", e), LssgErrorKind::Io))?;

    let original_width = img.width();
    let original_height = img.height();
    info!(
        "Successfully loaded image: {}x{}",
        original_width, original_height
    );

    let mut optimized_img = img;

    // Only resize if the image is both too large in dimensions AND has a significant file size
    // This prevents unnecessary quality loss on small images
    let should_resize = if let (Some(max_w), Some(max_h)) = (options.max_width, options.max_height)
    {
        let dimensions_too_large = optimized_img.width() > max_w || optimized_img.height() > max_h;
        let file_size_significant = data.len() > options.resize_threshold_bytes;
        dimensions_too_large && file_size_significant
    } else {
        false
    };

    if should_resize {
        let (max_w, max_h) = (options.max_width.unwrap(), options.max_height.unwrap());
        optimized_img = optimized_img.resize(max_w, max_h, image::imageops::FilterType::Lanczos3);
        info!(
            "Resized image {} from {}x{} to {}x{} (file size: {} bytes)",
            original_name,
            original_width,
            original_height,
            optimized_img.width(),
            optimized_img.height(),
            data.len()
        );
    } else if let (Some(max_w), Some(max_h)) = (options.max_width, options.max_height)
        && (optimized_img.width() > max_w || optimized_img.height() > max_h)
    {
        info!(
            "Skipping resize for {} ({}x{}, {} bytes) - file size too small to warrant quality loss",
            original_name,
            optimized_img.width(),
            optimized_img.height(),
            data.len()
        );
    }

    let mut buffer = Vec::new();
    let new_extension = if options.convert_to_webp {
        // Convert to WebP - convert the image to RGBA8 for webp crate compatibility
        let rgba_img = optimized_img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        let encoder = webp::Encoder::from_rgba(&rgba_img, width, height);
        let webp_data = encoder.encode(options.image_quality as f32).to_vec();
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
    let compression_ratio = if !data.is_empty() {
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
