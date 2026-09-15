mod engine;
#[cfg(target_os = "macos")]
mod ffi;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod service;

#[derive(Default)]
pub struct Options {
    pub run: bool,
    pub request_permissions: bool,
    pub dry_run: bool,
    pub window_title: Option<String>,
    pub duration: Option<f64>,
}
fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut opt = Options::default();
    let mut args = args;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => {}
            "--request-permissions" => opt.request_permissions = true,
            "--run" => opt.run = true,
            "--dry-run" => {
                opt.run = true;
                opt.dry_run = true;
            }
            "--window-title" => {
                let title = args.next().ok_or("--window-title requires a substring")?;
                if title.is_empty() {
                    return Err("window title must not be empty".into());
                }
                opt.window_title = Some(title);
            }
            "--duration" => {
                let n: f64 = args
                    .next()
                    .ok_or("--duration requires seconds")?
                    .parse()
                    .map_err(|_| "invalid duration")?;
                if !n.is_finite() || n <= 0.0 {
                    return Err("duration must be positive".into());
                }
                opt.duration = Some(n);
            }
            "--help" | "-h" => {
                println!("mackeyrelay\n\nstart [OPTIONS]: start a background service\nstop: stop the background service\nstatus: show service and last permission status\nautostart enable|disable: configure next-login startup\nlogs: show the last 80 log lines\n\nDefault / --check: read-only permissions and focused-window status\n--request-permissions: request macOS consent, then exit (never starts capture)\n--run: capture and forward keyboard while a remote window is focused\n--dry-run: observe routing only; do not suppress or inject events\n--window-title TEXT: require focused window title to contain TEXT\n--duration SECONDS: automatically stop after this interval\n\nEmergency stop: Control + Option + Shift + Escape\nOnly keyboard keyDown/keyUp/flagsChanged; media/system events are excluded.");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {a}")),
        }
    }
    if opt.request_permissions && opt.run {
        return Err("--request-permissions cannot be combined with --run or --dry-run".into());
    }
    Ok(opt)
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(target_os = "macos")]
    if args.first().is_some_and(|a| {
        matches!(
            a.as_str(),
            "start" | "stop" | "status" | "autostart" | "logs"
        )
    }) {
        if let Err(e) = service::dispatch(&args) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }
    let result: Result<(), String> = parse_args(args.into_iter()).and_then(|o| {
        #[cfg(target_os = "macos")]
        {
            macos::start(o)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = o;
            Err("This program requires macOS".into())
        }
    });
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1)
    }
}
