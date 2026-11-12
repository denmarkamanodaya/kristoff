//! int8 vector quantisation (Phase 3 §1.5): 384 × i8 plus one f32 scale per vector.

/// Quantises `v` symmetrically: `q[i] = round(v[i] / scale)` with `scale = max|v| / 127`.
///
/// Returns the bytes (two's-complement i8) and the scale. An all-zero vector gets scale 0.
#[must_use]
pub fn quantize(v: &[f32]) -> (Vec<u8>, f32) {
    let max = v.iter().fold(0f32, |m, x| m.max(x.abs()));
    if max == 0.0 {
        return (vec![0; v.len()], 0.0);
    }
    let scale = max / 127.0;
    let bytes = v
        .iter()
        .map(|x| {
            let q = (x / scale).round().clamp(-127.0, 127.0);
            // In range by the clamp above, so the cast neither truncates nor wraps.
            #[allow(clippy::cast_possible_truncation)]
            let q = q as i8;
            q.to_ne_bytes()[0]
        })
        .collect();
    (bytes, scale)
}

/// Dot product of an f32 query with a stored int8 vector and its scale.
#[must_use]
pub fn dot_query(query: &[f32], stored: &[u8], scale: f32) -> f32 {
    let raw: f32 = query
        .iter()
        .zip(stored)
        .map(|(q, b)| q * f32::from(i8::from_ne_bytes([*b])))
        .sum();
    raw * scale
}

/// Integer dot product of two int8 vectors stored as bytes.
#[must_use]
pub fn dot_i8(a: &[u8], b: &[u8]) -> i32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| i32::from(i8::from_ne_bytes([*x])) * i32::from(i8::from_ne_bytes([*y])))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantised_dot_tracks_the_float_dot() {
        let a: Vec<f32> = (0i16..384)
            .map(|i| f32::from(i * 7 % 23 - 11) / 40.0)
            .collect();
        let b: Vec<f32> = (0i16..384)
            .map(|i| f32::from(i * 5 % 19 - 9) / 30.0)
            .collect();
        let exact: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let ((qa, sa), (qb, sb)) = (quantize(&a), quantize(&b));

        let approx = f64::from(dot_i8(&qa, &qb)) * f64::from(sa) * f64::from(sb);

        let tolerance = 0.02 * f64::from(exact.abs().max(1.0));
        assert!(
            (approx - f64::from(exact)).abs() < tolerance,
            "{approx} vs {exact}"
        );
        assert!((dot_query(&a, &qb, sb) - exact).abs() < 0.02 * exact.abs().max(1.0));
    }
}
