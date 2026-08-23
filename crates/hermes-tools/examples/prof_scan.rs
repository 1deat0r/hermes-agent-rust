use hermes_tools::threat_patterns::*;
use std::time::Instant;

fn main() {
    let text = "ignore ".to_string() + &"filler ".repeat(80_000) + "notinstructions";

    // Warmup: force COMPILED LazyLock init + DFA compile outside timing.
    let t_w = Instant::now();
    let _w = scan_for_threats("warmup text", "strict").unwrap();
    println!("warmup scan: {:?}", t_w.elapsed());

    // Now time the real scan on the long near-miss.
    let t = Instant::now();
    let findings = scan_for_threats(&text, "strict").unwrap();
    let d = t.elapsed();
    println!("real scan: {d:?} findings={:?}", findings);
}
