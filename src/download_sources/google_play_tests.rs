// Validate the real shared Clap definition without calling request_aas_token,
// download_apps, login, or accept_tos. All values are synthetic placeholders.
use crate::cli;

#[test]
fn oauth_and_ini_arguments_parse_without_requiring_an_app_or_outpath() {
    let matches = cli::app()
        .try_get_matches_from([
            "apkeep",
            "--oauth-token",
            "FAKE_OAUTH_NOT_A_CREDENTIAL",
            "-e",
            "offline@example.invalid",
            "-i",
            "fixture.ini",
        ])
        .unwrap();
    assert_eq!(
        matches.get_one::<String>("google_oauth_token").unwrap(),
        "FAKE_OAUTH_NOT_A_CREDENTIAL"
    );
    assert_eq!(
        matches.get_one::<String>("google_email").unwrap(),
        "offline@example.invalid"
    );
    assert_eq!(matches.get_one::<String>("ini").unwrap(), "fixture.ini");
    assert!(matches.get_one::<String>("app").is_none());
    assert!(matches.get_one::<String>("OUTPATH").is_none());
    assert!(!matches.get_flag("google_accept_tos"));
}

#[test]
fn aas_and_auth_arguments_are_distinct_and_do_not_implicitly_accept_tos() {
    for switch in ["--aas-token", "--auth-token"] {
        let matches = cli::app()
            .try_get_matches_from([
                "apkeep",
                "-a",
                "com.example.fixture",
                "-e",
                "offline@example.invalid",
                switch,
                "FAKE_TOKEN_NOT_A_CREDENTIAL",
                "-i",
                "fixture.ini",
                ".",
            ])
            .unwrap();
        let (key, other) = if switch == "--aas-token" {
            ("google_aas_token", "google_auth_token")
        } else {
            ("google_auth_token", "google_aas_token")
        };
        assert_eq!(
            matches.get_one::<String>(key).unwrap(),
            "FAKE_TOKEN_NOT_A_CREDENTIAL"
        );
        assert!(matches.get_one::<String>(other).is_none());
        assert_eq!(matches.get_one::<String>("ini").unwrap(), "fixture.ini");
        assert!(!matches.get_flag("google_accept_tos"));
    }
}
