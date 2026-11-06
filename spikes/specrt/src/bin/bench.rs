use anyhow::{Context, Result};
use specrt::runtime::Runtime;
use specrt::spec::Spec;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let spec_path = PathBuf::from(args.next().context("usage: bench <spec.toml> [prompt_len] [new_tokens]")?);
    let prompt_len: usize = args.next().map(|s| s.parse()).transpose()?.unwrap_or(128);
    let new_tokens: usize = args.next().map(|s| s.parse()).transpose()?.unwrap_or(128);
    let spec = Spec::load(&spec_path)?;
    let started = std::time::Instant::now();
    let mut rt = Runtime::load(&spec, prompt_len + new_tokens)?;
    let load_secs = started.elapsed().as_secs_f64();
    let prompt: Vec<u32> = (0..prompt_len as u32).map(|i| 1000 + i * 7).collect();
    rt.generate(&prompt, 8.min(new_tokens))?;
    let g = rt.generate(&prompt, new_tokens)?;
    println!(
        "{} quant={:?} arch={} jit_cache_hit={} load={:.2}s prompt={} prefill={:.1} tok/s decode={} tokens {:.1} tok/s",
        spec.name,
        spec.quant,
        rt.arch,
        rt.cache_hit,
        load_secs,
        prompt_len,
        prompt_len as f64 / g.prefill_secs,
        new_tokens,
        (new_tokens - 1) as f64 / g.decode_secs,
    );
    Ok(())
}
