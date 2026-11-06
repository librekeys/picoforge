//! Persistent CCID (PC/SC) session for the smart-card applets.
//!
//! Unlike [`super::pcsc::PcscTransport`] (one fresh connection per rescue op),
//! a `CcidSession` **holds the card open** across calls. That is mandatory for
//! the applets: `SELECT` resets the card's security status, so a VERIFY and the
//! operation it authorises must run on the same open card. It also grows the rx
//! buffer past 256 bytes and implements `61xx`/`6Cxx` response assembly and
//! `CLA|0x10` command-chaining, none of which the rescue transport has.

// `send_chained` is consumed by the PIV/OpenPGP import stages, not by OATH.
#![allow(dead_code)]

use crate::error::PFError;
use crate::hal::apdu::{
    Apdu, CLA_CHAIN, CLA_ISO, INS_GET_RESPONSE, INS_SELECT, INS_SEND_REMAINING, StatusWord,
};
use pcsc::{Context, Protocols, Scope, ShareMode};

/// Largest single PC/SC response chunk we read before assembling via GET RESPONSE.
const RX_BUF: usize = 4096;
/// Largest command-data fragment in a chained write (ISO short Lc max).
const CHAIN_CHUNK: usize = 255;

/// An open PC/SC card bound to one applet (selected by AID).
pub struct CcidSession {
    card: pcsc::Card,
    /// The applet's `SELECT` response (FCI / version block), parsed per applet.
    pub select_resp: Vec<u8>,
}

impl CcidSession {
    /// Connect to the first reader and `SELECT` the given applet AID.
    ///
    /// The card is kept open for the session's lifetime so a subsequent VERIFY
    /// stays in effect for the following operation.
    pub fn open(aid: &[u8]) -> Result<Self, PFError> {
        let ctx = Context::establish(Scope::User).map_err(PFError::Pcsc)?;
        let mut readers_buf = [0; 2048];
        let reader = ctx
            .list_readers(&mut readers_buf)?
            .next()
            .ok_or(PFError::NoDevice)?;
        let card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;

        let mut session = Self {
            card,
            select_resp: Vec::new(),
        };
        let select = Apdu::read(CLA_ISO, INS_SELECT, 0x04, 0x00, aid);
        session.select_resp = session.transceive_full(&select).map_err(|e| {
            PFError::Device(format!(
                "Applet not available — enable it and the CCID interface in Configuration ({e})"
            ))
        })?;
        Ok(session)
    }

    /// Send one APDU, returning `(response_data, status_word)`.
    pub fn transceive(&self, apdu: &Apdu) -> Result<(Vec<u8>, StatusWord), PFError> {
        let tx = apdu.encode();
        let mut rx = [0u8; RX_BUF];
        let resp = self.card.transmit(&tx, &mut rx).map_err(PFError::Pcsc)?;
        if resp.len() < 2 {
            return Err(PFError::Device("Truncated APDU response".into()));
        }
        let (data, sw) = resp.split_at(resp.len() - 2);
        Ok((
            data.to_vec(),
            StatusWord(u16::from_be_bytes([sw[0], sw[1]])),
        ))
    }

    /// Send an APDU and assemble the full response across `61xx` continuations,
    /// retrying once on `6Cxx`. Errors on any non-`9000` final SW. Continues a
    /// `61xx` page with ISO GET RESPONSE (`0xC0`) — the right form for every
    /// applet except OATH (see [`transceive_oath`](Self::transceive_oath)).
    pub fn transceive_full(&self, apdu: &Apdu) -> Result<Vec<u8>, PFError> {
        self.transceive_paged(apdu, INS_GET_RESPONSE)
    }

    /// Like [`transceive_full`](Self::transceive_full) but continues a `61xx`
    /// page with YKOATH SEND REMAINING (`0xA5`) instead of GET RESPONSE. The
    /// OATH applet paginates LIST / CALCULATE ALL this way and rejects `0xC0`
    /// with `6D00`, discarding the pending page — so its two paged reads must
    /// use this. (Matches a real YubiKey, which pages OATH identically.)
    pub fn transceive_oath(&self, apdu: &Apdu) -> Result<Vec<u8>, PFError> {
        self.transceive_paged(apdu, INS_SEND_REMAINING)
    }

    fn transceive_paged(&self, apdu: &Apdu, continue_ins: u8) -> Result<Vec<u8>, PFError> {
        let mut cmd = apdu.clone();
        let mut out = Vec::new();
        loop {
            let (data, sw) = self.transceive(&cmd)?;
            if let Some(n) = sw.wrong_le() {
                // 6Cxx: resend the same command with the corrected Le, no data yet.
                cmd.le = Some(if n == 0 { 256 } else { n as u16 });
                continue;
            }
            out.extend_from_slice(&data);
            if let Some(n) = sw.more_data() {
                cmd = Apdu::read(CLA_ISO, continue_ins, 0, 0, &[]);
                cmd.le = Some(if n == 0 { 256 } else { n as u16 });
                continue;
            }
            if sw.is_ok() {
                return Ok(out);
            }
            return Err(sw.to_error());
        }
    }

    /// Send a command whose data exceeds 255 bytes via ISO command-chaining
    /// (`CLA|0x10` on every fragment but the last). Used for PIV/OpenPGP import.
    pub fn send_chained(&self, apdu: &Apdu) -> Result<Vec<u8>, PFError> {
        if apdu.data.len() <= CHAIN_CHUNK {
            return self.transceive_full(apdu);
        }
        let data = apdu.data.clone();
        let mut i = 0;
        while data.len() - i > CHAIN_CHUNK {
            let frag = Apdu::write(
                apdu.cla | CLA_CHAIN,
                apdu.ins,
                apdu.p1,
                apdu.p2,
                &data[i..i + CHAIN_CHUNK],
            );
            let (_, sw) = self.transceive(&frag)?;
            if !sw.is_ok() {
                return Err(sw.to_error());
            }
            i += CHAIN_CHUNK;
        }
        let last = Apdu {
            cla: apdu.cla,
            ins: apdu.ins,
            p1: apdu.p1,
            p2: apdu.p2,
            data: data[i..].to_vec(),
            le: apdu.le,
        };
        self.transceive_full(&last)
    }
}
