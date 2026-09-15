//! Per-user launchd management. Never enables automatic restart.
use std::fmt::Write as _;
use std::io::{Read, Seek, SeekFrom, Write as _};
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}
use std::{env, fs, path::PathBuf, process::Command, thread, time::Duration};
const LABEL: &str = "io.mackeyrelay.agent";
struct Paths {
    root: PathBuf,
    binary: PathBuf,
    manual: PathBuf,
    login: PathBuf,
    log: PathBuf,
    domain: String,
}
impl Paths {
    fn new() -> Result<Self, String> {
        let uid = unsafe { crate::ffi::geteuid() };
        if uid == 0 {
            return Err("Run as the logged-in desktop user, without sudo".into());
        }
        let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is unavailable")?);
        let root = home.join("Library/Application Support/MacKeyRelay");
        Ok(Self {
            binary: root.join("mackeyrelay"),
            manual: root.join("agent.plist"),
            login: home.join("Library/LaunchAgents/io.mackeyrelay.agent.plist"),
            log: root.join("service.log"),
            root,
            domain: format!("gui/{uid}"),
        })
    }
    fn target(&self) -> String {
        format!("{}/{LABEL}", self.domain)
    }
    fn info(&self) -> Result<Option<String>, String> {
        let out = Command::new("/bin/launchctl")
            .args(["print", &self.target()])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned()))
        } else if String::from_utf8_lossy(&out.stderr).contains("Could not find service") {
            Ok(None)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().into())
        }
    }
    fn install(&self) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let current = env::current_exe().map_err(|e| e.to_string())?;
        let bytes = fs::read(&current).map_err(|e| e.to_string())?;
        if fs::read(&self.binary).ok().as_ref() != Some(&bytes) {
            let tmp = self.root.join("mackeyrelay.new");
            fs::copy(current, &tmp).map_err(|e| e.to_string())?;
            fs::rename(tmp, &self.binary).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn write_plist(&self, path: &std::path::Path, args: &[String]) -> Result<(), String> {
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(
            path,
            plist(
                &self.binary.to_string_lossy(),
                &self.log.to_string_lossy(),
                args,
            ),
        )
        .map_err(|e| e.to_string())
    }
    fn registration(&self, enabled: bool) -> Result<(), String> {
        if enabled {
            self.write_plist(&self.login, &["--run".into()])
        } else {
            match fs::remove_file(&self.login) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        }
    }
    fn stop(&self) -> Result<(), String> {
        if self.info()?.is_some() {
            ctl(&["bootout", &self.target()])?;
        }
        Ok(())
    }
    fn logs(&self) -> Result<(), String> {
        match log_tail(&self.log) {
            Ok(text) => {
                let lines: Vec<_> = text.lines().collect();
                for line in &lines[lines.len().saturating_sub(80)..] {
                    println!("{line}");
                }
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("No service logs yet");
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }
}
fn service_summary(info: &str) -> Vec<&str> {
    info.lines()
        .map(str::trim)
        .filter(|l| {
            [
                "state =",
                "pid =",
                "last exit code =",
                "last exit reason =",
                "last terminating signal =",
            ]
            .iter()
            .any(|key| l.starts_with(key))
        })
        .collect()
}
fn log_tail(path: &std::path::Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(65536)))?;
    let mut bytes = Vec::new();
    file.take(65536).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
fn ctl(args: &[&str]) -> Result<(), String> {
    let out = Command::new("/bin/launchctl")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "launchctl: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn plist(binary: &str, log: &str, args: &[String]) -> String {
    let mut arguments = String::new();
    for arg in std::iter::once(binary).chain(args.iter().map(String::as_str)) {
        write!(arguments, "<string>{}</string>", xml(arg)).expect("writing to String");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{LABEL}</string>
<key>ProgramArguments</key><array>{arguments}</array>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><false/>
<key>LimitLoadToSessionType</key><string>Aqua</string>
<key>ExitTimeOut</key><integer>5</integer>
<key>StandardOutPath</key><string>{}</string>
<key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
        xml(log),
        xml(log)
    )
}
pub fn dispatch(args: &[String]) -> Result<(), String> {
    let p = Paths::new()?;
    // Serialize management commands so simultaneous starts cannot replace each other's job.
    let _lock = if args
        .first()
        .is_some_and(|a| matches!(a.as_str(), "start" | "stop" | "autostart"))
    {
        fs::create_dir_all(&p.root).map_err(|e| e.to_string())?;
        fs::set_permissions(&p.root, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(p.root.join("management.lock"))
            .map_err(|e| e.to_string())?;
        if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
            return Err("Another service management command is in progress; retry shortly".into());
        }
        Some(file)
    } else {
        None
    };
    match args.first().map(String::as_str) {
        Some("start") => {
            let options = crate::parse_args(args[1..].iter().cloned())?;
            if options.request_permissions { return Err("Request permissions separately in the foreground".into()); }
            if p.info()?.is_some_and(|s| s.lines().any(|l| l.trim() == "state = running")) {
                return Err("Service already running; stop it before changing options".into());
            }
            p.stop()?;
            p.install()?;
            let mut run_args = vec!["--run".into()]; run_args.extend_from_slice(&args[1..]);
            p.write_plist(&p.manual, &run_args)?;
            fs::write(&p.log, "Starting background service\n").map_err(|e| e.to_string())?;
            ctl(&["bootstrap", &p.domain, p.manual.to_str().ok_or("Invalid plist path")?])?;
            // Check actual background permissions/readiness, not those of the launcher.
            for _ in 0..30 {
                let log = log_tail(&p.log).unwrap_or_default();
                if log.contains("capture enabled") || log.contains("dry-run (no forwarding)") {
                    println!("Background service started. Use mackeyrelay status or stop."); return Ok(());
                }
                if log.contains("error:") { p.logs()?; p.stop()?; return Err(format!("Background startup failed. Grant Accessibility and Input Monitoring to {} in System Settings, then run start again. Terminal consent may not apply.", p.binary.display())); }
                thread::sleep(Duration::from_millis(100));
            }
            p.logs()?;
            let info = p.info()?.unwrap_or_default();
            for line in service_summary(&info) { println!("{line}"); }
            let mut log = fs::OpenOptions::new().append(true).open(&p.log).map_err(|e| e.to_string())?;
            for line in service_summary(&info) { writeln!(log, "startup: {line}").map_err(|e| e.to_string())?; }
            p.stop()?;
            Err("Background service did not report ready and was unloaded. If the exit reason is OS_REASON_ENDPOINTSECURITY, execution was blocked by endpoint security; contact your administrator.".into())
        }
        Some("stop") if args.len() == 1 => { p.stop()?; println!("Background service stopped (login preference unchanged)"); Ok(()) }
        Some("status") if args.len() == 1 => {
            println!("autostart: {}", if p.login.exists() { "enabled" } else { "disabled" });
            match p.info()? {
                None => println!("service: not loaded"),
                Some(info) => for line in service_summary(&info) { println!("{line}"); },
            }
            println!("Last background attempt (not a live permission check):");
            if let Ok(log) = log_tail(&p.log) {
                for line in log.lines().filter(|l| l.starts_with("permissions:") || l.starts_with("error:") || l.starts_with("startup:")).rev().take(6) { println!("{line}"); }
            }
            Ok(())
        }
        Some("logs") if args.len() == 1 => p.logs(),
        Some("autostart") if args.len() == 2 => match args[1].as_str() {
            "enable" => {
                if p.info()?.is_some_and(|s| s.lines().any(|l| l.trim() == "state = running")) && !p.binary.exists() { return Err("Stop the service before enabling autostart".into()); }
                // Do not replace a running executable during login registration.
                if !p.info()?.is_some_and(|s| s.lines().any(|l| l.trim() == "state = running")) { p.install()?; }
                p.registration(true)?;
                println!("Login autostart enabled for the next login; use start to run now. Login runs use default forwarding options."); Ok(())
            }
            "disable" => { p.registration(false)?; println!("Login autostart disabled; use stop to stop the current service"); Ok(()) }
            _ => Err("Usage: mackeyrelay autostart enable|disable".into()),
        },
        _ => Err("Usage: mackeyrelay start [--window-title TEXT] [--duration SECONDS] [--dry-run] | stop | status | logs | autostart enable|disable".into()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_registration_roundtrip_preserves_manual_configuration() {
        let root = env::temp_dir().join(format!("mackeyrelay-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let p = Paths {
            binary: root.join("program"),
            manual: root.join("manual.plist"),
            login: root.join("login.plist"),
            log: root.join("log"),
            root: root.clone(),
            domain: "unused".into(),
        };
        p.write_plist(
            &p.manual,
            &["--dry-run".into(), "--duration".into(), "1".into()],
        )
        .unwrap();
        let before = fs::read(&p.manual).unwrap();
        p.registration(true).unwrap();
        let login = fs::read_to_string(&p.login).unwrap();
        assert!(login.contains("<string>--run</string>"));
        assert!(!login.contains("--dry-run"));
        assert!(Command::new("/usr/bin/plutil")
            .arg("-lint")
            .arg(&p.login)
            .output()
            .unwrap()
            .status
            .success());
        p.registration(false).unwrap();
        p.registration(false).unwrap();
        assert!(!p.login.exists());
        assert_eq!(fs::read(&p.manual).unwrap(), before);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn diagnostics_exclude_environment_and_include_security_failure() {
        let info = "state = not running\n inherited environment = {\n PRIVATE = value\n}\nlast exit reason = OS_REASON_ENDPOINTSECURITY";
        assert_eq!(
            service_summary(info),
            vec![
                "state = not running",
                "last exit reason = OS_REASON_ENDPOINTSECURITY"
            ]
        );
    }
    #[test]
    fn launch_config_escapes_arguments_and_never_restarts() {
        let s = plist(
            "/example/a & b/tool",
            "/example/log",
            &["--window-title".into(), "<remote>\"'".into()],
        );
        assert!(s.contains("a &amp; b"));
        assert!(s.contains("&lt;remote&gt;&quot;&apos;"));
        assert!(s.contains("<key>KeepAlive</key><false/>"));
        assert!(s.contains("<key>RunAtLoad</key><true/>"));
    }
}
