//!
//! Command line interface parsing
//!
#![forbid(unsafe_code)]
#![deny(missing_docs)]
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
};
use log::debug;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

// Basic checks for valid filesystem path
fn is_valid_filesystem_path(s: String) -> Result<(), String> {
    debug!("Ensuring that output.file-path is valid");

    // - is special and is a request for us to output to stdout
    if s == "-" {
        return Ok(())
    }

    // Get a Path from our string and start checking
    let path = Path::new(&s);

    // We only take absolute paths
    if !path.is_absolute() {
        return Err("output.file-path only accepts absolute paths".to_owned());
    }

    // We can't write to a directory
    if path.is_dir() {
        return Err("output.file-path must not point at a directory".to_owned());
    }

    // Node Exporter textfiles must end with .prom
    if let Some(ext) = path.extension() {
        // Got an extension, ensure that it's .prom
        if ext != "prom" {
            return Err("output.file-path must have .prom extension".to_owned());
        }
    }
    else {
        // Didn't find an extension at all
        return Err("output.file-path must have .prom extension".to_owned());
    }

    // Check that the directory exists
    if let Some(dir) = path.parent() {
        // Got a parent directory, ensure it exists
        if !dir.is_dir() {
            return Err("output.file-path directory must exist".to_owned());
        }
    }
    else {
        // Didn't get a parent directory at all
        return Err("output.file-path directory must exist".to_owned());
    }

    Ok(())
}

// Used as a validator for the argument parsing.
fn is_valid_socket_addr(s: String) -> Result<(), String> {
    debug!("Ensuring that web.listen-address is valid");

    match SocketAddr::from_str(&s) {
        Ok(_)  => Ok(()),
        Err(_) => Err(format!("'{}' is not a valid ADDR:PORT string", s)),
    }
}

// Checks that the telemetry_path is valid.
// This check is extremely basic, and there may still be invalid paths that
// could be passed.
fn is_valid_telemetry_path(s: String) -> Result<(), String> {
    debug!("Ensuring that web.telemetry-path is valid");

    // Ensure s isn't empty.
    if s.is_empty() {
        return Err("path must not be empty".to_owned());
    }

    // Ensure that s starts with /
    if !s.starts_with('/') {
        return Err("path must start with /".to_owned());
    }

    // Ensure that s isn't literally /
    if s == "/" {
        return Err("path must not be /".to_owned());
    }

    Ok(())
}

// Create a clap app
fn create_app<'a, 'b>() -> clap::App<'a, 'b> {
    debug!("Creating clap app");

    clap::App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .set_term_width(80)
        .arg(
            clap::Arg::with_name("OUTPUT_FILE_PATH")
                .env("JAIL_EXPORTER_OUTPUT_FILE_PATH")
                .hide_env_values(true)
                .long("output.file-path")
                .value_name("FILE")
                .help("File to output metrics to.")
                .takes_value(true)
                .validator(is_valid_filesystem_path)
        )
        .arg(
            clap::Arg::with_name("WEB_LISTEN_ADDRESS")
                .env("JAIL_EXPORTER_WEB_LISTEN_ADDRESS")
                .hide_env_values(true)
                .long("web.listen-address")
                .value_name("[ADDR:PORT]")
                .help("Address on which to expose metrics and web interface.")
                .takes_value(true)
                .default_value("127.0.0.1:9452")
                .validator(is_valid_socket_addr)
        )
        .arg(
            clap::Arg::with_name("WEB_TELEMETRY_PATH")
                .env("JAIL_EXPORTER_WEB_TELEMETRY_PATH")
                .hide_env_values(true)
                .long("web.telemetry-path")
                .value_name("PATH")
                .help("Path under which to expose metrics.")
                .takes_value(true)
                .default_value("/metrics")
                .validator(is_valid_telemetry_path)
        )
}

// Parses the command line arguments and returns the matches.
pub fn parse_args<'a>() -> clap::ArgMatches<'a> {
    debug!("Parsing command line arguments");

    create_app().get_matches()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use pretty_assertions::assert_eq;
    use std::env;
    use std::panic;
    use std::sync::Mutex;

    lazy_static! {
        // Used during env_tests
        static ref LOCK: Mutex<i8> = Mutex::new(0);
    }

    // Wraps setting and unsetting of environment variables
    fn env_test<T>(key: &str, var: &str, test: T) -> ()
    where T: FnOnce() -> () + panic::UnwindSafe {
        // This ensures that only one test can be manipulating the environment
        // at a time.
        let _locked = LOCK.lock().unwrap();

        env::set_var(key, var);

        let result = panic::catch_unwind(|| {
            test()
        });

        env::remove_var(key);

        assert!(result.is_ok())
    }

    #[test]
    fn default_web_listen_address() {
        // Must lock since we're still testing env vars here even though we're
        // not setting one.
        let _locked = LOCK.lock().unwrap();

        let argv = vec!["jail_exporter"];
        let matches = create_app().get_matches_from(argv);
        let listen_address = matches.value_of("WEB_LISTEN_ADDRESS");

        assert_eq!(listen_address, Some("127.0.0.1:9452"));
    }

    #[test]
    fn default_web_telemetry_path() {
        // Must lock since we're still testing env vars here even though we're
        // not setting one.
        let _locked = LOCK.lock().unwrap();

        let argv = vec!["jail_exporter"];
        let matches = create_app().get_matches_from(argv);
        let telemetry_path = matches.value_of("WEB_TELEMETRY_PATH");

        assert_eq!(telemetry_path, Some("/metrics"));
    }

    #[test]
    fn cli_set_web_listen_address() {
        let argv = vec![
            "jail_exporter",
            "--web.listen-address=127.0.1.2:9452",
        ];

        let matches = create_app().get_matches_from(argv);
        let listen_address = matches.value_of("WEB_LISTEN_ADDRESS");

        assert_eq!(listen_address, Some("127.0.1.2:9452"));
    }

    #[test]
    fn cli_override_env_web_listen_address() {
        env_test("JAIL_EXPORTER_WEB_LISTEN_ADDRESS", "127.0.1.2:9452", || {
            let argv = vec![
                "jail_exporter",
                "--web.listen-address=127.0.1.3:9452",
            ];

            let matches = create_app().get_matches_from(argv);
            let listen_address = matches.value_of("WEB_LISTEN_ADDRESS");

            assert_eq!(listen_address, Some("127.0.1.3:9452"));
        });
    }

    #[test]
    fn cli_override_env_web_telemetry_path() {
        env_test("JAIL_EXPORTER_WEB_TELEMETRY_PATH", "/envvar", || {
            let argv = vec![
                "jail_exporter",
                "--web.telemetry-path=/clioverride",
            ];

            let matches = create_app().get_matches_from(argv);
            let listen_address = matches.value_of("WEB_TELEMETRY_PATH");

            assert_eq!(listen_address, Some("/clioverride"));
        });
    }

    #[test]
    fn cli_set_web_telemetry_path() {
        let argv = vec![
            "jail_exporter",
            "--web.telemetry-path=/test",
        ];

        let matches = create_app().get_matches_from(argv);
        let telemetry_path = matches.value_of("WEB_TELEMETRY_PATH");

        assert_eq!(telemetry_path, Some("/test"));
    }

    #[test]
    fn env_set_web_listen_address() {
        env_test("JAIL_EXPORTER_WEB_LISTEN_ADDRESS", "127.0.1.2:9452", || {
            let argv = vec!["jail_exporter"];
            let matches = create_app().get_matches_from(argv);
            let listen_address = matches.value_of("WEB_LISTEN_ADDRESS");

            assert_eq!(listen_address, Some("127.0.1.2:9452"));
        });
    }

    #[test]
    fn env_set_web_telemetry_path() {
        env_test("JAIL_EXPORTER_WEB_TELEMETRY_PATH", "/test", || {
            let argv = vec!["jail_exporter"];
            let matches = create_app().get_matches_from(argv);
            let telemetry_path = matches.value_of("WEB_TELEMETRY_PATH");

            assert_eq!(telemetry_path, Some("/test"));
        });
    }

    #[test]
    fn is_valid_filesystem_path_absolute_path() {
        let res = is_valid_filesystem_path("tmp/metrics.prom".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_bad_extension() {
        let res = is_valid_filesystem_path("/tmp/metrics.pram".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_bad_parent_dir() {
        let res = is_valid_filesystem_path("/tmp/nope/metrics.prom".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_directory() {
        let res = is_valid_filesystem_path("/tmp".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_no_extension() {
        let res = is_valid_filesystem_path("/tmp/metrics".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_ok() {
        let res = is_valid_filesystem_path("/tmp/metrics.prom".into());
        assert!(res.is_ok());
    }

    #[test]
    fn is_valid_filesystem_path_root() {
        let res = is_valid_filesystem_path("/".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_filesystem_path_stdout() {
        let res = is_valid_filesystem_path("-".into());
        assert!(res.is_ok());
    }

    #[test]
    fn is_valid_socket_addr_ipv4_with_port() {
        let res = is_valid_socket_addr("127.0.0.1:9452".into());
        assert!(res.is_ok());
    }

    #[test]
    fn is_valid_socket_addr_ipv6_with_port() {
        let res = is_valid_socket_addr("[::1]:9452".into());
        assert!(res.is_ok());
    }

    #[test]
    fn is_valid_socket_addr_ipv4_without_port() {
        let res = is_valid_socket_addr("127.0.0.1".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_socket_addr_ipv6_without_port() {
        let res = is_valid_socket_addr("[::1]".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_socket_addr_no_ip() {
        let res = is_valid_socket_addr("random string".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_telemetry_path_slash() {
        let res = is_valid_telemetry_path("/".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_telemetry_path_empty() {
        let res = is_valid_telemetry_path("".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_telemetry_path_relative() {
        let res = is_valid_telemetry_path("metrics".into());
        assert!(res.is_err());
    }

    #[test]
    fn is_valid_telemetry_path_valid() {
        let res = is_valid_telemetry_path("/metrics".into());
        assert!(res.is_ok());
    }
}
