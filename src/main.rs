use std::env;
use std::thread;
use std::time::{Duration, Instant};
use image::RgbaImage;
use xcap::Monitor;
use enigo::{Enigo, Mouse, Settings, Coordinate, Button, Direction};

// Custom lightweight Grayscale Image struct for fast pixel access
struct GrayscaleImage {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl GrayscaleImage {
    fn from_rgba(img: &RgbaImage) -> Self {
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut data = Vec::with_capacity(width * height);
        for pixel in img.pixels() {
            // Standard luma formula: Y = 0.299*R + 0.587*G + 0.114*B
            let gray = (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) as u8;
            data.push(gray);
        }
        Self { width, height, data }
    }
}

// Template struct storing search image, alpha mask, and statistical properties
struct Template {
    width: usize,
    height: usize,
    gray: Vec<u8>,
    mask_indices: Vec<(usize, usize)>,
    mean: f32,
    variance: f32,
    std_dev: f32,
}

impl Template {
    fn new(img: &RgbaImage) -> Self {
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut gray = Vec::with_capacity(width * height);
        let mut mask_indices = Vec::new();

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x as u32, y as u32);
                let g = (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) as u8;
                let is_active = pixel[3] > 128; // Alpha threshold
                gray.push(g);
                if is_active {
                    mask_indices.push((x, y));
                }
            }
        }

        // Calculate mean of active pixels
        let n = mask_indices.len() as f32;
        let mut sum = 0.0;
        for &(x, y) in &mask_indices {
            sum += gray[y * width + x] as f32;
        }
        let mean = if n > 0.0 { sum / n } else { 0.0 };

        // Calculate variance
        let mut variance = 0.0;
        for &(x, y) in &mask_indices {
            let diff = gray[y * width + x] as f32 - mean;
            variance += diff * diff;
        }
        let std_dev = variance.sqrt();

        Self {
            width,
            height,
            gray,
            mask_indices,
            mean,
            variance,
            std_dev,
        }
    }
}

// Zero-Mean Normalized Cross-Correlation (ZNCC) with Masking
fn zncc_score(screen: &GrayscaleImage, x: usize, y: usize, template: &Template) -> f32 {
    let n = template.mask_indices.len() as f32;
    if n == 0.0 || template.std_dev == 0.0 {
        return 0.0;
    }

    // 1. Calculate mean of screen patch under mask
    let mut sum_w = 0.0;
    for &(dx, dy) in &template.mask_indices {
        sum_w += screen.data[(y + dy) * screen.width + (x + dx)] as f32;
    }
    let mean_w = sum_w / n;

    // 2. Calculate variance and covariance
    let mut var_w = 0.0;
    let mut cov = 0.0;
    for &(dx, dy) in &template.mask_indices {
        let val_w = screen.data[(y + dy) * screen.width + (x + dx)] as f32;
        let val_t = template.gray[dy * template.width + dx] as f32;
        let diff_w = val_w - mean_w;
        let diff_t = val_t - template.mean;
        var_w += diff_w * diff_w;
        cov += diff_w * diff_t;
    }

    if var_w <= 0.001 {
        return 0.0;
    }

    let den = (var_w * template.variance).sqrt();
    if den <= 0.001 {
        0.0
    } else {
        cov / den
    }
}

// Simple helper to draw a border on RGB image for debugging
fn draw_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: [u8; 3]) {
    let rgba_color = image::Rgba([color[0], color[1], color[2], 255]);
    let thickness = 6; // Make it thick enough to see on high-DPI displays
    
    for t in 0..thickness {
        // Top and bottom edges
        for dx in 0..w {
            let px = x + dx;
            let py1 = y + t;
            if px < img.width() && py1 < img.height() {
                img.put_pixel(px, py1, rgba_color);
            }
            if h > t {
                let py2 = (y + h).saturating_sub(1 + t);
                if px < img.width() && py2 < img.height() {
                    img.put_pixel(px, py2, rgba_color);
                }
            }
        }
        // Left and right edges
        for dy in 0..h {
            let py = y + dy;
            let px1 = x + t;
            if px1 < img.width() && py < img.height() {
                img.put_pixel(px1, py, rgba_color);
            }
            if w > t {
                let px2 = (x + w).saturating_sub(1 + t);
                if px2 < img.width() && py < img.height() {
                    img.put_pixel(px2, py, rgba_color);
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let test_mode = args.len() > 1 && args[1] == "--test";

    println!("Starting Golden Cookie Clicker...");
    if test_mode {
        println!("*** RUNNING IN TEST MODE (Will not click, will save diagnostic images) ***");
    }

    // Load cookie template
    let cookie_bytes = include_bytes!("../cookie.png");
    let cookie_rgba = image::load_from_memory(cookie_bytes)
        .expect("Failed to load embedded cookie.png")
        .to_rgba8();

    // Define search scales
    let scales = vec![0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5];

    // Initialize Enigo mouse controller
    println!("Initializing Enigo...");
    let mut enigo = if !test_mode {
        match Enigo::new(&Settings::default()) {
            Ok(e) => {
                println!("Enigo initialized successfully.");
                Some(e)
            }
            Err(err) => {
                eprintln!("Error initializing mouse controller (Enigo): {:?}", err);
                eprintln!("Please check OS Accessibility permissions.");
                return;
            }
        }
    } else {
        println!("Skipping Enigo initialization in test mode.");
        None
    };
    println!("Continuing main loop...");

    let check_interval = Duration::from_secs(2);

    loop {
        let loop_start = Instant::now();

        // Get all monitors
        let monitors = match Monitor::all() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error getting monitors: {:?}", e);
                thread::sleep(check_interval);
                continue;
            }
        };

        for monitor in &monitors {
            // Capture monitor screen
            let screenshot = match monitor.capture_image() {
                Ok(img) => img,
                Err(e) => {
                    eprintln!("Error capturing monitor {}: {:?}", monitor.name(), e);
                    continue;
                }
            };

            let screen_w = screenshot.width();
            let screen_h = screenshot.height();

            // Downscale screenshot by 8x for coarse search (speeds up search and supports larger scale ranges)
            let coarse_w = screen_w / 8;
            let coarse_h = screen_h / 8;
            let screenshot_coarse = image::imageops::resize(
                &screenshot,
                coarse_w,
                coarse_h,
                image::imageops::FilterType::Triangle,
            );
            let screenshot_coarse_gray = GrayscaleImage::from_rgba(&screenshot_coarse);

            let mut overall_best_refined_score = -1.0;
            let mut overall_best_x = 0;
            let mut overall_best_y = 0;
            let mut overall_best_scale = 1.0;

            let screenshot_gray = GrayscaleImage::from_rgba(&screenshot);

            // 1. Search across scales
            for &scale in &scales {
                let target_w = ((cookie_rgba.width() as f32 * scale) as u32).max(8);
                let target_h = ((cookie_rgba.height() as f32 * scale) as u32).max(8);

                // Downscale template by 8x for coarse search
                let c_w = (target_w / 8).max(6);
                let c_h = (target_h / 8).max(6);

                let template_coarse_img = image::imageops::resize(
                    &cookie_rgba,
                    c_w,
                    c_h,
                    image::imageops::FilterType::Nearest, // Nearest is best for templates to keep binary transparency mask
                );
                let template_coarse = Template::new(&template_coarse_img);

                if screenshot_coarse_gray.width < template_coarse.width || screenshot_coarse_gray.height < template_coarse.height {
                    continue;
                }

                let max_y = screenshot_coarse_gray.height - template_coarse.height;
                let max_x = screenshot_coarse_gray.width - template_coarse.width;

                let mut scale_best_coarse_score = -1.0;
                let mut scale_best_coarse_x = 0;
                let mut scale_best_coarse_y = 0;

                // Coarse search (8x downscaled space, step 1)
                for y in 0..=max_y {
                    for x in 0..=max_x {
                        let score = zncc_score(&screenshot_coarse_gray, x, y, &template_coarse);
                        if score > scale_best_coarse_score {
                            scale_best_coarse_score = score;
                            scale_best_coarse_x = x;
                            scale_best_coarse_y = y;
                        }
                    }
                }

                // If coarse candidate looks promising, run fine refinement on 1x image for this scale
                if scale_best_coarse_score > 0.30 {
                    let template_fine_img = image::imageops::resize(
                        &cookie_rgba,
                        target_w,
                        target_h,
                        image::imageops::FilterType::Nearest,
                    );
                    let template_fine = Template::new(&template_fine_img);

                    let orig_approx_x = scale_best_coarse_x * 8;
                    let orig_approx_y = scale_best_coarse_y * 8;

                    let search_range = 16; // +/- 16 pixels at 1x to cover the 8x coarse cell
                    let start_x = orig_approx_x.saturating_sub(search_range);
                    let end_x = (orig_approx_x + search_range).min(screen_w as usize - template_fine.width);
                    let start_y = orig_approx_y.saturating_sub(search_range);
                    let end_y = (orig_approx_y + search_range).min(screen_h as usize - template_fine.height);

                    let mut scale_best_refined_score = -1.0;
                    let mut scale_best_refined_x = orig_approx_x;
                    let mut scale_best_refined_y = orig_approx_y;

                    for y in start_y..=end_y {
                        for x in start_x..=end_x {
                            let score = zncc_score(&screenshot_gray, x, y, &template_fine);
                            if score > scale_best_refined_score {
                                scale_best_refined_score = score;
                                scale_best_refined_x = x;
                                scale_best_refined_y = y;
                            }
                        }
                    }

                    if scale_best_refined_score > overall_best_refined_score {
                        overall_best_refined_score = scale_best_refined_score;
                        overall_best_x = scale_best_refined_x;
                        overall_best_y = scale_best_refined_y;
                        overall_best_scale = scale;
                    }
                }
            }

            let detection_threshold = 0.70;
            let is_detected = overall_best_refined_score >= detection_threshold;

            if is_detected || test_mode {
                let target_w = ((cookie_rgba.width() as f32 * overall_best_scale) as u32).max(8);
                let target_h = ((cookie_rgba.height() as f32 * overall_best_scale) as u32).max(8);
                let template_fine_img = image::imageops::resize(
                    &cookie_rgba,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Nearest,
                );
                let template_fine = Template::new(&template_fine_img);

                let fine_best_x = overall_best_x;
                let fine_best_y = overall_best_y;
                let best_scale = overall_best_scale;

                // Final coordinates in physical pixels
                let center_x_pixels = fine_best_x + template_fine.width / 2;
                let center_y_pixels = fine_best_y + template_fine.height / 2;

                // Scale factor for logical coordinates
                let scale_factor = monitor.scale_factor() as f64;
                let logical_x = (center_x_pixels as f64 / scale_factor) as i32;
                let logical_y = (center_y_pixels as f64 / scale_factor) as i32;

                // Absolute screen coordinates
                let abs_x = monitor.x() + logical_x;
                let abs_y = monitor.y() + logical_y;

                let fine_detected = overall_best_refined_score >= detection_threshold;

                if fine_detected {
                    println!(
                        "Cookie DETECTED on '{}'! Score: {:.3} (scale: {:.1}) | Center: ({}, {})px -> Absolute logical: ({}, {})",
                        monitor.name(),
                        overall_best_refined_score,
                        best_scale,
                        center_x_pixels,
                        center_y_pixels,
                        abs_x,
                        abs_y
                    );
                } else {
                    println!(
                        "No cookie detected on '{}'. Best candidate score: {:.3} (scale: {:.1}) | Center: ({}, {})px -> Absolute logical: ({}, {})",
                        monitor.name(),
                        overall_best_refined_score,
                        best_scale,
                        center_x_pixels,
                        center_y_pixels,
                        abs_x,
                        abs_y
                    );
                }

                if test_mode {
                    // Save diagnostic images
                    let mut debug_screenshot = screenshot.clone();
                    let color = if fine_detected {
                        [0, 255, 0] // Green box for successful detection
                    } else {
                        [0, 0, 255] // Blue box for best candidate (below threshold)
                    };

                    draw_rect(
                        &mut debug_screenshot,
                        fine_best_x as u32,
                        fine_best_y as u32,
                        template_fine.width as u32,
                        template_fine.height as u32,
                        color,
                    );
                    let filename = format!("match_result_{}.png", monitor.name().replace(' ', "_"));
                    match debug_screenshot.save(&filename) {
                        Ok(_) => println!("Saved diagnostic image: {}", filename),
                        Err(e) => eprintln!("Failed to save diagnostic image: {:?}", e),
                    }
                } else if fine_detected {
                    if let Some(ref mut e) = enigo {
                        println!("Moving mouse to ({}, {}) and clicking...", abs_x, abs_y);
                        if let Err(err) = e.move_mouse(abs_x, abs_y, Coordinate::Abs) {
                            eprintln!("Mouse move error: {:?}", err);
                        } else {
                            thread::sleep(Duration::from_millis(50));
                            if let Err(err) = e.button(Button::Left, Direction::Click) {
                                eprintln!("Mouse click error: {:?}", err);
                            } else {
                                println!("Click triggered successfully!");
                            }
                        }
                    }
                }
            }
        }

        let elapsed = loop_start.elapsed();
        if test_mode {
            println!("Search loop completed in {:?}", elapsed);
            // In test mode, we run once and exit
            println!("Test mode complete. Exiting.");
            break;
        }

        // Sleep to avoid burning energy (2 seconds total loop interval)
        if elapsed < check_interval {
            thread::sleep(check_interval - elapsed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn test_cookie_detection() {
        // 1. Load embedded template
        let cookie_bytes = include_bytes!("../cookie.png");
        let cookie_rgba = image::load_from_memory(cookie_bytes)
            .expect("Failed to load embedded cookie.png")
            .to_rgba8();

        let t_w = cookie_rgba.width();
        let t_h = cookie_rgba.height();

        // 2. Create a mock screenshot (RGBA) of size 1000x1000
        let mut mock_screenshot = RgbaImage::from_pixel(1000, 1000, Rgba([240, 240, 240, 255]));

        // 3. Paste the cookie template at a specific location, say (350, 450)
        let paste_x = 350;
        let paste_y = 450;
        for y in 0..t_h {
            for x in 0..t_w {
                let pixel = cookie_rgba.get_pixel(x, y);
                let alpha = pixel[3] as f32 / 255.0;
                let bg_pixel = mock_screenshot.get_pixel(paste_x + x, paste_y + y);
                
                // Simple alpha blending
                let r = (pixel[0] as f32 * alpha + bg_pixel[0] as f32 * (1.0 - alpha)) as u8;
                let g = (pixel[1] as f32 * alpha + bg_pixel[1] as f32 * (1.0 - alpha)) as u8;
                let b = (pixel[2] as f32 * alpha + bg_pixel[2] as f32 * (1.0 - alpha)) as u8;
                
                mock_screenshot.put_pixel(paste_x + x, paste_y + y, Rgba([r, g, b, 255]));
            }
        }

        // 4. Run the coarse search (4x downscaled)
        let screen_w = mock_screenshot.width();
        let screen_h = mock_screenshot.height();
        let coarse_w = screen_w / 4;
        let coarse_h = screen_h / 4;
        let screenshot_coarse = image::imageops::resize(
            &mock_screenshot,
            coarse_w,
            coarse_h,
            image::imageops::FilterType::Triangle,
        );
        let screenshot_coarse_gray = GrayscaleImage::from_rgba(&screenshot_coarse);

        let scales = vec![0.8, 1.0, 1.2];
        let mut best_score = -1.0;
        let mut best_coarse_x = 0;
        let mut best_coarse_y = 0;
        let mut best_scale = 1.0;

        for &scale in &scales {
            let target_w = ((cookie_rgba.width() as f32 * scale) as u32).max(8);
            let target_h = ((cookie_rgba.height() as f32 * scale) as u32).max(8);

            let c_w = (target_w / 4).max(8);
            let c_h = (target_h / 4).max(8);

            let template_coarse_img = image::imageops::resize(
                &cookie_rgba,
                c_w,
                c_h,
                image::imageops::FilterType::Triangle,
            );
            let template_coarse = Template::new(&template_coarse_img);

            let max_y = screenshot_coarse_gray.height - template_coarse.height;
            let max_x = screenshot_coarse_gray.width - template_coarse.width;

            for y in (0..=max_y).step_by(2) {
                for x in (0..=max_x).step_by(2) {
                    let score = zncc_score(&screenshot_coarse_gray, x, y, &template_coarse);
                    if score > best_score {
                        best_score = score;
                        best_coarse_x = x;
                        best_coarse_y = y;
                        best_scale = scale;
                    }
                }
            }
        }

        // Assert that the coarse search found it near the target location (350 / 4 = 87, 450 / 4 = 112)
        let expected_coarse_x = paste_x as usize / 4;
        let expected_coarse_y = paste_y as usize / 4;
        
        // Coarse coordinates should be within +/- 2 pixels
        assert!((best_coarse_x as isize - expected_coarse_x as isize).abs() <= 2, "Coarse X offset too large");
        assert!((best_coarse_y as isize - expected_coarse_y as isize).abs() <= 2, "Coarse Y offset too large");
        assert!((best_scale - 1.0).abs() < 0.01, "Coarse scale mismatch");

        // 5. Fine refinement
        let target_w = ((cookie_rgba.width() as f32 * best_scale) as u32).max(8);
        let target_h = ((cookie_rgba.height() as f32 * best_scale) as u32).max(8);
        let template_fine_img = image::imageops::resize(
            &cookie_rgba,
            target_w,
            target_h,
            image::imageops::FilterType::Triangle,
        );
        let template_fine = Template::new(&template_fine_img);

        let orig_approx_x = best_coarse_x * 4;
        let orig_approx_y = best_coarse_y * 4;

        let search_range = 8;
        let start_x = orig_approx_x.saturating_sub(search_range);
        let end_x = (orig_approx_x + search_range).min(screen_w as usize - template_fine.width);
        let start_y = orig_approx_y.saturating_sub(search_range);
        let end_y = (orig_approx_y + search_range).min(screen_h as usize - template_fine.height);

        let screenshot_gray = GrayscaleImage::from_rgba(&mock_screenshot);

        let mut fine_best_score = -1.0;
        let mut fine_best_x = orig_approx_x;
        let mut fine_best_y = orig_approx_y;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let score = zncc_score(&screenshot_gray, x, y, &template_fine);
                if score > fine_best_score {
                    fine_best_score = score;
                    fine_best_x = x;
                    fine_best_y = y;
                }
            }
        }

        // Assert that the fine search found it exactly at (350, 450) and score is very high (close to 1.0)
        assert_eq!(fine_best_x, paste_x as usize, "Fine X coordinate mismatch");
        assert_eq!(fine_best_y, paste_y as usize, "Fine Y coordinate mismatch");
        assert!(fine_best_score > 0.95, "Fine match score too low: {}", fine_best_score);
        
        println!("Test passed! Cookie successfully detected at exact location with score: {}", fine_best_score);
    }

    #[test]
    fn check_saved_image_pixels() {
        if let Ok(img) = image::open("match_result_Built-in_Retina_Display.png") {
            let rgba = img.to_rgba8();
            let mut blue_count = 0;
            let mut green_count = 0;
            for pixel in rgba.pixels() {
                if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 255 && pixel[3] == 255 {
                    blue_count += 1;
                }
                if pixel[0] == 0 && pixel[1] == 255 && pixel[2] == 0 && pixel[3] == 255 {
                    green_count += 1;
                }
            }
            println!("Blue pixels: {}, Green pixels: {}", blue_count, green_count);
            assert!(blue_count > 0 || green_count > 0, "No boundary pixels found in the image!");
        }
    }
}

