use rip2::args::{validate_args, Args};
use rip2::util::TestMode;
use rip2::{copy_file, move_target};
use rstest::rstest;
use std::fs;
use std::io::Cursor;
use std::os::unix;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process;
use tempfile::tempdir;

#[cfg(target_os = "macos")]
use std::os::unix::fs::FileTypeExt;

#[rstest]
fn test_validation() {
    let bad_completions = Args {
        completions: Some("bash".to_string()),
        decompose: true,
        ..Args::default()
    };
    validate_args(&bad_completions).expect_err("--completions can only be used by itself");

    let bad_decompose = Args {
        decompose: true,
        seance: true,
        ..Args::default()
    };
    validate_args(&bad_decompose).expect_err("-d,--decompose can only be used with --graveyard");
}

#[rstest]
fn test_filetypes(
    #[values("regular", "big", "fifo", "symlink", "socket")] file_type: &str,
    #[values(false, true)] copy: bool,
) {
    if ["big", "socket"].contains(&file_type) && !copy {
        return;
    }
    let tmpdir = tempdir().unwrap();
    let path = PathBuf::from(tmpdir.path());
    let source_path = path.join("test_file");
    let dest_path = path.join("test_file_copy");

    match file_type {
        "regular" => {
            fs::File::create(&source_path).unwrap();
        }
        "big" => {
            let file = fs::File::create(&source_path).unwrap();
            let len = rip2::BIG_FILE_THRESHOLD + 1;
            file.set_len(len).unwrap();
        }
        "fifo" => {
            process::Command::new("mkfifo")
                .arg(&source_path)
                .output()
                .unwrap();
        }
        "symlink" => {
            let target_path = path.join("symlink_target");
            fs::File::create(&target_path).unwrap();
            unix::fs::symlink(&target_path, &source_path).unwrap();
        }
        "socket" => {
            UnixListener::bind(&source_path).unwrap();
        }
        _ => unreachable!(),
    }

    let mut log = Vec::new();
    let mode = TestMode;

    if copy {
        copy_file(&source_path, &dest_path, &mode, &mut log).unwrap();
    } else {
        move_target(&source_path, &dest_path, &mode, &mut log).unwrap();
    }

    let log_s = String::from_utf8(log).unwrap();

    // Check logs
    match file_type {
        "big" => {
            assert!(log_s.contains("About to copy a big file"));
        }
        "socket" => {
            assert!(log_s.contains("Non-regular file or directory:"));
            assert!(log_s.contains("Permanently delete the file?"));
        }
        _ => {
            assert!(log_s.is_empty())
        }
    }

    // Check graveyard contents and file type
    // let metadata = fs::symlink_metadata(dest_path).unwrap();
    // let ftype = metadata.file_type();
    let ftype = fs::symlink_metadata(&dest_path).map(|m| m.file_type());
    match file_type {
        "regular" => {
            assert!(dest_path.is_file());
            assert!(ftype.unwrap().is_file());
        }
        "big" => {
            assert!(!dest_path.exists());
        }
        "fifo" => {
            #[cfg(target_os = "macos")]
            {
                assert!(dest_path.exists());
                assert!(ftype.unwrap().is_fifo());
                // TODO: Why does this fail on Linux?
            }
        }
        "symlink" => {
            assert!(dest_path.exists());
            assert!(ftype.unwrap().is_symlink());
        }
        "socket" => {
            assert!(dest_path.exists());
            assert!(ftype.unwrap().is_file());
            let contents = fs::read_to_string(&dest_path).unwrap();
            assert!(contents.contains("marker for a file that was permanently deleted."));
        }
        _ => {}
    }
}

#[rstest]
fn test_prompt_read(
    #[values(
        ("y", true),
        ("Y", true),
        ("n", false),
        ("q", false),
    )]
    key: (&str, bool),
) {
    let input = Cursor::new(key.0);
    let result = rip2::util::process_in_stream(input);
    assert_eq!(result, key.1)
}
