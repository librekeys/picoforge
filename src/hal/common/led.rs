//! Shared parsing for the RS-Key LED status block.
//!
//! Both the CCID Vendor/LED applet (`GET LED 0x11`) and the FIDO CTAPHID
//! `0x41 CONFIG_READ` (target LED) return the same `EF_LED_CONF` block. Its
//! current layout is `[steady, (effect, color, brightness, speed) × 4]`
//! (17 bytes); older firmware returns a 13-byte (pre-speed) or 9-byte
//! (pre-effect) block. The per-status stride is `(len - 1) / 4`, mirroring
//! RS-Key's own `rsk_led::load_block`. Only `color` and `brightness` are
//! surfaced to the config UI.

/// Number of device-status slots (idle, processing, touch, boot).
const N_STATUS: usize = 4;

/// Parse an `EF_LED_CONF` block into `(steady, [(color, brightness); 4])`.
///
/// `data` is the raw config block with no CBOR wrapper or status-word suffix.
/// Returns `None` when the block is too short to hold four status records
/// (stride `< 2`), so a malformed/truncated read fails cleanly rather than
/// reporting garbage colours.
pub fn parse_led_block(data: &[u8]) -> Option<(bool, [(u8, u8); N_STATUS])> {
    if data.is_empty() {
        return None;
    }

    let stride = (data.len() - 1) / N_STATUS;
    if stride < 2 {
        return None;
    }

    // color then brightness sit right after the optional leading effect byte:
    // stride >= 3 (with effect) puts them at record offset +1/+2, the pre-effect
    // stride-2 block starts with colour at offset +0.
    let color_off = if stride >= 3 { 1 } else { 0 };
    let steady = data[0] != 0;
    let mut statuses = [(0u8, 0u8); N_STATUS];
    for (i, slot) in statuses.iter_mut().enumerate() {
        let base = 1 + stride * i + color_off;
        *slot = (*data.get(base)?, *data.get(base + 1)?);
    }

    Some((steady, statuses))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_17_byte_block() {
        // [steady, (effect, color, brightness, speed) × 4]
        let block = [
            0x01, // steady
            0x00, 0x02, 0x40, 0x00, // idle:  green, br 0x40
            0x01, 0x03, 0x20, 0x05, // proc:  blue,  br 0x20
            0x02, 0x04, 0x10, 0x0F, // touch: yellow, br 0x10
            0x00, 0x01, 0x08, 0x00, // boot:  red,   br 0x08
        ];
        let (steady, statuses) = parse_led_block(&block).unwrap();
        assert!(steady);
        assert_eq!(statuses, [(2, 0x40), (3, 0x20), (4, 0x10), (1, 0x08)]);
    }

    #[test]
    fn effect_byte_is_not_mistaken_for_colour() {
        // Regression: a 17-byte block whose effect bytes differ from the colours.
        // The old stride-2 parse read the effect byte as the colour.
        let mut block = [0u8; 17];
        block[0] = 0; // steady = false
        for i in 0..4 {
            block[1 + 4 * i] = 0x03; // effect (would be misread as colour)
            block[2 + 4 * i] = (i as u8) + 1; // colour 1..=4
            block[3 + 4 * i] = 0x11 * (i as u8 + 1); // brightness
            block[4 + 4 * i] = 0x00; // speed
        }
        let (steady, statuses) = parse_led_block(&block).unwrap();
        assert!(!steady);
        assert_eq!(statuses, [(1, 0x11), (2, 0x22), (3, 0x33), (4, 0x44)]);
    }

    #[test]
    fn parses_pre_speed_13_byte_block() {
        // [steady, (effect, color, brightness) × 4]
        let block = [
            0x00, // steady
            0x00, 0x02, 0x40, // idle
            0x00, 0x03, 0x20, // proc
            0x00, 0x04, 0x10, // touch
            0x00, 0x01, 0x08, // boot
        ];
        let (steady, statuses) = parse_led_block(&block).unwrap();
        assert!(!steady);
        assert_eq!(statuses, [(2, 0x40), (3, 0x20), (4, 0x10), (1, 0x08)]);
    }

    #[test]
    fn parses_pre_effect_9_byte_block() {
        // [steady, (color, brightness) × 4]
        let block = [
            0x01, // steady
            0x02, 0x40, // idle
            0x03, 0x20, // proc
            0x04, 0x10, // touch
            0x01, 0x08, // boot
        ];
        let (steady, statuses) = parse_led_block(&block).unwrap();
        assert!(steady);
        assert_eq!(statuses, [(2, 0x40), (3, 0x20), (4, 0x10), (1, 0x08)]);
    }

    #[test]
    fn rejects_blocks_too_short_for_four_statuses() {
        assert!(parse_led_block(&[]).is_none());
        assert!(parse_led_block(&[0x01]).is_none());
        assert!(parse_led_block(&[0x01, 0x02, 0x40, 0x03, 0x20, 0x04, 0x10, 0x01]).is_none());
    }
}
