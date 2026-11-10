pub fn gaussian_like_blur(data: &mut [u8], width: usize, height: usize, std_dev: f32) {
    if std_dev <= 0.0 {
        return;
    }
    const PASSES: u32 = 1;
    let ideal_width = ((12.0 * std_dev * std_dev / PASSES as f32) + 1.0).sqrt();
    let radius = (((ideal_width - 1.0) / 2.0).round() as i64).max(1);

    let inner_max = width.max(height);
    let mut line = vec![0i64; inner_max * 4];
    let mut out_line = vec![0u8; inner_max * 4];

    for _ in 0..PASSES {
        box_blur_1d(data, width, height, radius, true, &mut line, &mut out_line);
        box_blur_1d(data, width, height, radius, false, &mut line, &mut out_line);
    }
}

fn box_blur_1d(
    data: &mut [u8],
    width: usize,
    height: usize,
    radius: i64,
    horizontal: bool,
    line: &mut [i64],
    out_line: &mut [u8],
) {
    let window = 2 * radius + 1;
    let (outer, inner) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let line = &mut line[..inner * 4];
    let out_line = &mut out_line[..inner * 4];

    for o in 0..outer {
        for i in 0..inner {
            let (x, y) = if horizontal { (i, o) } else { (o, i) };
            let idx = (y * width + x) * 4;
            for c in 0..4 {
                line[i * 4 + c] = data[idx + c] as i64;
            }
        }

        let mut sums = [0i64; 4];
        for k in 0..=radius {
            if (k as usize) < inner {
                for c in 0..4 {
                    sums[c] += line[k as usize * 4 + c];
                }
            }
        }

        for i in 0..inner {
            for c in 0..4 {
                out_line[i * 4 + c] = (sums[c] / window).clamp(0, 255) as u8;
            }
            let add_idx = i as i64 + radius + 1;
            let remove_idx = i as i64 - radius;
            if add_idx >= 0 && (add_idx as usize) < inner {
                for c in 0..4 {
                    sums[c] += line[add_idx as usize * 4 + c];
                }
            }
            if remove_idx >= 0 && (remove_idx as usize) < inner {
                for c in 0..4 {
                    sums[c] -= line[remove_idx as usize * 4 + c];
                }
            }
        }

        for i in 0..inner {
            let (x, y) = if horizontal { (i, o) } else { (o, i) };
            let idx = (y * width + x) * 4;
            data[idx..idx + 4].copy_from_slice(&out_line[i * 4..i * 4 + 4]);
        }
    }
}
