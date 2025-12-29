# Media Module

The Media module provides automatic optimization for images and videos in your static site. It can resize, compress, and convert media files to improve page load times and reduce bandwidth usage.

## Options

All media options are configured under the `[media]` section:

### Image Optimization

#### `optimize_images`
- **Type:** Boolean
- **Default:** `true`
- **Description:** Enable automatic image optimization during build.

#### `image_quality`
- **Type:** Integer (1-100)
- **Default:** `85`
- **Description:** JPEG/PNG compression quality. Higher values mean better quality but larger file sizes.

#### `convert_to_webp`
- **Type:** Boolean
- **Default:** `true`
- **Description:** Convert images to WebP format for better compression while maintaining quality.

#### `webp_quality`
- **Type:** Integer (1-100)
- **Default:** `95`
- **Description:** WebP compression quality. Values of 95 or higher use lossless compression.

#### `max_width`
- **Type:** Integer (pixels)
- **Default:** `1920`
- **Optional:** Set to `null` to disable
- **Description:** Maximum width for images. Images larger than this will be resized.

#### `max_height`
- **Type:** Integer (pixels)
- **Default:** `1080`
- **Optional:** Set to `null` to disable
- **Description:** Maximum height for images. Images larger than this will be resized.

#### `resize_threshold_bytes`
- **Type:** Integer (bytes)
- **Default:** `1000000` (1 MB)
- **Description:** Minimum file size to trigger resizing. Small images won't be resized even if they exceed max dimensions, preventing unnecessary quality loss.

### Video Optimization

#### `optimize_videos`
- **Type:** Boolean
- **Default:** `true`
- **Description:** Enable automatic video optimization during build.

#### `use_ffmpeg`
- **Type:** Boolean
- **Default:** `true`
- **Description:** Use FFmpeg for video optimization. FFmpeg must be installed on your system.

#### `video_crf`
- **Type:** Integer (0-51)
- **Default:** `25`
- **Description:** Constant Rate Factor for video encoding. Lower values mean better quality but larger files. Range: 0 (lossless) to 51 (worst quality). Recommended range: 18-28.

## Example Configuration

### Conservative Settings (High Quality)

```markdown
<!--
[media]
optimize_images = true
image_quality = 95
convert_to_webp = false
max_width = 2560
max_height = 1440
resize_threshold_bytes = 2000000
webp_quality = 98

optimize_videos = true
video_crf = 18
-->
```

### Aggressive Optimization (Smaller Files)

```markdown
<!--
[media]
optimize_images = true
image_quality = 75
convert_to_webp = true
max_width = 1280
max_height = 720
resize_threshold_bytes = 500000
webp_quality = 85

optimize_videos = true
video_crf = 28
-->
```

### Disable Optimization

```markdown
<!--
[media]
optimize_images = false
optimize_videos = false
-->
```

## How It Works

### Image Processing

1. **Load:** Image is loaded and parsed
2. **Resize:** If image exceeds `max_width` or `max_height` AND file size exceeds `resize_threshold_bytes`, it's resized while maintaining aspect ratio
3. **Format Conversion:** If `convert_to_webp` is enabled, the image is converted to WebP format
4. **Compression:** Image is compressed according to `image_quality` or `webp_quality`
5. **Output:** Optimized image replaces the original in the build output

### Video Processing

1. **Detection:** Videos are detected by file extension (mp4, webm, ogg, ogv, mov, avi)
2. **FFmpeg:** If available and enabled, videos are re-encoded using the H.264 codec
3. **CRF:** The Constant Rate Factor controls the quality/size tradeoff
4. **Output:** Optimized video replaces the original

## Requirements

### For Image Optimization
- No external dependencies (uses Rust `image` crate)
- Supports: JPEG, PNG, GIF, BMP, TIFF, WebP

### For Video Optimization
- **FFmpeg** must be installed on your system
- FFmpeg must be in your system PATH
- Supported input formats: MP4, WebM, OGG, OGV, MOV, AVI
- Output format: MP4 (H.264)

## Installation of FFmpeg

### Linux (Ubuntu/Debian)
```bash
sudo apt install ffmpeg
```

### macOS
```bash
brew install ffmpeg
```

### Windows
Download from [ffmpeg.org](https://ffmpeg.org/download.html) and add to PATH.

## Quality Guidelines

### Image Quality

- **95-100:** Nearly lossless, very large files
- **85-94:** High quality, good balance (recommended)
- **75-84:** Good quality, noticeable compression
- **60-74:** Acceptable quality, significant compression
- **Below 60:** Poor quality, not recommended

### Video CRF

- **18-22:** Very high quality, large files
- **23-25:** High quality, reasonable files (recommended)
- **26-28:** Good quality, smaller files
- **29-32:** Acceptable quality, significant compression
- **Above 32:** Poor quality, not recommended

## Performance Tips

1. **Selective Optimization:** Disable optimization for pages where you need original quality
2. **Threshold Tuning:** Adjust `resize_threshold_bytes` to avoid processing small images
3. **WebP:** Use WebP for better compression, but be aware of older browser compatibility
4. **Batch Processing:** The module processes all media during the build phase

## Troubleshooting

### "FFmpeg not found"
- Install FFmpeg using the instructions above
- Verify installation: `ffmpeg -version`
- Or disable video optimization: `optimize_videos = false`

### Images look too compressed
- Increase `image_quality` or `webp_quality`
- Consider using `convert_to_webp = false` for PNG images with transparency

### Build is slow
- Reduce `max_width` and `max_height`
- Increase `resize_threshold_bytes` to skip small images
- Disable optimization for development builds

### Large file sizes
- Reduce `image_quality` or `webp_quality`
- Decrease `max_width` and `max_height`
- Ensure `convert_to_webp = true` for better compression
- Increase `video_crf` for smaller video files