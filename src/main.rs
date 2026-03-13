use anyhow::{Context, Result, bail, ensure};
use hound::WavReader;
use itertools::Itertools;

fn main() -> Result<()> {
    // let mut r = WavReader::open(dbg!("apple-hello-cut.wav"))?;
    let mut r = WavReader::open(dbg!("apple-hello.wav"))?;

    let samples: Vec<_> = r.samples::<i16>().try_collect()?;
    dbg!(&samples);

    let bits = samples_to_bits(&samples)?;

    let bytes = bits_to_bytes(&bits)?;
    dbg!(&bytes);
    let mut checksum = 0xff;
    for byte in bytes {
        checksum ^= byte;

        let c = byte & 0x7f;
        print!("{}", c as char);
    }
    println!();
    dbg!(checksum); // 0

    Ok(())
}

// skip silence at beginning
// skip through symmetric values in header tone
// detect asymmetric sync bit

fn samples_to_bits(samples: &[i16]) -> Result<Vec<bool>> {
    ensure!(!samples.is_empty());

    // trim silence at beginning
    let max_amplitude = samples.iter().map(|s| s.abs()).max().context("max")?;
    let samples: Vec<_> = samples
        .iter()
        .skip_while(|&&s| s < max_amplitude / 8)
        .copied()
        .collect();

    let runs: Vec<_> = samples
        .chunk_by(|x, y| x.signum() == y.signum())
        .filter(|chunk| chunk[0] != 0)
        .map(|chunk| chunk.len())
        .collect();
    ensure!(!runs.is_empty());

    // skip past header tone and "sync bit"
    let sync_bit = find_sync_bit(&runs)?;
    ensure!(sync_bit + 2 < runs.len());
    let runs = &runs[sync_bit + 2..];

    // pretend we know the length
    let known_num_bits = 6 * 8;
    let num_runs = known_num_bits * 2;
    ensure!(runs.len() >= num_runs);
    let runs = &runs[..num_runs];

    ensure!(runs.len() % 2 == 0); // (else trimming was wrong)

    let min = runs.iter().min().context("min")?;
    let max = runs.iter().max().context("max")?;
    let avg = (max + min) / 2;

    runs.chunks(2)
        .map(|pair| {
            let is_long = match (pair[0] >= avg, pair[1] >= avg) {
                (true, true) => true,
                (false, false) => false,
                _ => bail!("mismatch"),
            };
            Ok(is_long) // long is '1'
        })
        .collect()
}

/// Return the index.
fn find_sync_bit(runs: &[usize]) -> Result<usize> {
    for i in 0..runs.len().saturating_sub(1) {
        if runs[i + 1] < runs[i] / 2 {
            return Ok(i + 1);
        }
    }
    bail!("couldn't find sync bit");
}

fn bits_to_bytes(bits: &[bool]) -> Result<Vec<u8>> {
    ensure!(bits.len() % 8 == 0);
    let mut out = vec![];
    for chunk in bits.chunks(8) {
        let mut byte = 0;
        for &bit in chunk {
            byte <<= 1;
            if bit {
                byte |= 1;
            }
        }
        out.push(byte);
    }
    Ok(out)
}
