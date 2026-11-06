//! A terminal resource probe; no native window or event loop is needed.
#[path = "resources/workload.rs"]
mod workload;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let number = |name: &str, default| -> Result<usize, Box<dyn std::error::Error>> {
        args.iter()
            .position(|arg| arg == name)
            .map_or(Ok(default), |i| {
                Ok(args.get(i + 1).ok_or("Missing numeric argument")?.parse()?)
            })
    };
    let report = workload::run(
        number("--rows", 100_000)?,
        number("--visits", 64)?,
        number("--edits", 2_000)?,
    )?;
    println!(
        "Rows per model: {} · visits: {} · edits: {}",
        report.rows, report.visits, report.edits
    );
    println!("{report:#?}");
    Ok(())
}
