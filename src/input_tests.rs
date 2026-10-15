//! Characterization of shared input helpers; fixtures never contain real credentials.
use super::*;

#[test]
fn csv_single_column_handles_crlf_blank_rows_and_final_newline() {
    assert_eq!(
        parse_csv_text("com.example.one\r\n\ncom.example.two\n".to_owned(), 1, None),
        vec![
            ("com.example.one".to_owned(), None),
            ("com.example.two".to_owned(), None)
        ]
    );
}

#[test]
fn csv_selects_app_and_version_columns_in_either_order() {
    assert_eq!(
        parse_csv_text("label,com.example.one,1.2.3".to_owned(), 2, Some(3)),
        vec![("com.example.one".to_owned(), Some("1.2.3".to_owned()))]
    );
    assert_eq!(
        parse_csv_text("1.2.3,label,com.example.one".to_owned(), 3, Some(1)),
        vec![("com.example.one".to_owned(), Some("1.2.3".to_owned()))]
    );
}

#[test]
fn csv_explicit_empty_version_means_latest() {
    assert_eq!(
        parse_csv_text("com.example.one,".to_owned(), 1, Some(2)),
        vec![("com.example.one".to_owned(), None)]
    );
}

#[test]
fn csv_reader_preserves_app_version_and_reports_missing_or_non_utf8_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("apps.csv");
    assert!(fetch_csv_list(path.to_str().unwrap(), 1, None).is_err());
    fs::write(&path, b"\xff").unwrap();
    assert!(fetch_csv_list(path.to_str().unwrap(), 1, None).is_err());
    fs::write(&path, b"com.example.one,2\ncom.example.two,3\n").unwrap();
    assert_eq!(
        fetch_csv_list(path.to_str().unwrap(), 1, Some(2)).unwrap(),
        vec![
            ("com.example.one".to_owned(), Some("2".to_owned())),
            ("com.example.two".to_owned(), Some("3".to_owned()))
        ]
    );
}

#[test]
fn explicit_ini_is_read_without_modifying_its_contents() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("apkeep.ini");
    let bytes = b"[google]\nemail=fixture@example.invalid\naas_token=fixture-only\n";
    fs::write(&path, bytes).unwrap();
    let config = load_config(Some(path.clone())).unwrap();
    assert_eq!(
        config.get("google", "email").as_deref(),
        Some("fixture@example.invalid")
    );
    assert_eq!(
        config.get("google", "aas_token").as_deref(),
        Some("fixture-only")
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn explicit_ini_reports_missing_malformed_and_non_utf8_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("apkeep.ini");
    assert!(load_config(Some(path.clone())).is_err());
    fs::write(&path, b"[unclosed-section\n").unwrap();
    assert!(load_config(Some(path.clone())).is_err());
    fs::write(&path, b"\xff").unwrap();
    assert!(load_config(Some(path)).is_err());
}
