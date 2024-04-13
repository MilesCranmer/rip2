use assert_cmd::Command;
use rand::distributions::Alphanumeric;
use rand::Rng;
use rip2::args::Args;
use rip2::util::TestMode;
use rip2::{self, util};
use rstest::rstest;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::{env, ffi, iter};
use tempfile::{tempdir, TempDir};

use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_LOCK: Mutex<()> = Mutex::new(());
}

fn aquire_lock() -> MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap()
}

struct TestEnv {
    _tmpdir: TempDir,
    graveyard: PathBuf,
    src: PathBuf,
}

impl TestEnv {
    fn new() -> TestEnv {
        let _tmpdir = tempdir().unwrap();
        let tmpdir_pathbuf = PathBuf::from(_tmpdir.path());
        let graveyard = tmpdir_pathbuf.join("graveyard");
        let src = tmpdir_pathbuf.join("data");

        // The graveyard should be created, so we don't test this:
        // fs::create_dir_all(&graveyard).unwrap();
        fs::create_dir_all(&src).unwrap();

        TestEnv {
            _tmpdir,
            graveyard,
            src,
        }
    }
}

struct TestData {
    data: String,
    path: PathBuf,
}

impl TestData {
    fn new(test_env: &TestEnv, filename: Option<&str>) -> TestData {
        let data = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(100)
            .map(char::from)
            .collect::<String>();

        let taken_filename = filename.unwrap_or("test_file.txt");
        let path = test_env.src.join(taken_filename);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(data.as_bytes()).unwrap();

        TestData { data, path }
    }
}

/// Test that a file is buried and unburied correctly
/// Also checks that the graveyard is deleted when decompose is true
#[rstest]
fn test_bury_unbury(#[values(false, true)] decompose: bool, #[values(false, true)] inspect: bool) {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env, None);
    // And is now in the graveyard
    let expected_graveyard_path =
        util::join_absolute(&test_env.graveyard, test_data.path.canonicalize().unwrap());

    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            inspect,
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();
    if inspect {
        let log_s = String::from_utf8(log).unwrap();
        assert!(log_s.contains("100 bytes"));
    } else {
        assert!(log.is_empty())
    }

    // Verify that the file no longer exists
    assert!(!test_data.path.exists());

    // Verify that the graveyard exists
    assert!(test_env.graveyard.exists());
    assert!(expected_graveyard_path.exists());

    // with the right data
    let restored_data_from_grave = fs::read_to_string(&expected_graveyard_path).unwrap();
    assert_eq!(restored_data_from_grave, test_data.data);

    let mut log = Vec::new();
    rip2::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            decompose,
            unbury: if decompose { None } else { Some(Vec::new()) },
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();
    let log_s = String::from_utf8(log).unwrap();
    if decompose {
        assert!(log_s.contains("Really unlink the entire graveyard?"));
    } else {
        assert!(log_s.contains("Returned"));
    }

    if decompose {
        // Verify that the graveyard is completely deleted
        assert!(!test_env.graveyard.exists());
        // And that the file was not restored
        assert!(!test_data.path.exists());
    } else {
        // Verify that the file exists in the original location with the correct data
        assert!(test_data.path.exists());
        let restored_data = fs::read_to_string(&test_data.path).unwrap();
        assert_eq!(restored_data, test_data.data);
    }
}

const ENV_VARS: [&str; 2] = ["RIP_GRAVEYARD", "XDG_DATA_HOME"];

// Delete env vars and return them
// so we can restore them later
fn cache_and_remove_env_vars() -> [Option<String>; 2] {
    // This should be the same size as ENV_VARS
    ENV_VARS.map(|key| {
        // Check if env var exists
        let value = env::var(key).ok();
        env::remove_var(key);
        value
    })
}

fn restore_env_vars(default_env_vars: [Option<String>; 2]) {
    // Iterate over the default env vars and restore them
    ENV_VARS
        .iter()
        .zip(default_env_vars.iter())
        .for_each(|(key, value)| {
            env::remove_var(key);
            if let Some(value) = value {
                env::set_var(key, value);
            }
        });
}

/// Test that we can set the graveyard from different env variables
#[rstest]
fn test_env(#[values("RIP_GRAVEYARD", "XDG_DATA_HOME")] env_var: &str) {
    let _env_lock = aquire_lock();

    let default_env_vars = cache_and_remove_env_vars();
    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env, None);
    let modified_graveyard = if env_var == "XDG_DATA_HOME" {
        // XDG version adds a "graveyard" folder
        util::join_absolute(&test_env.graveyard, "graveyard")
    } else {
        test_env.graveyard.clone()
    };
    let expected_graveyard_path =
        util::join_absolute(modified_graveyard, test_data.path.canonicalize().unwrap());

    let graveyard = test_env.graveyard.clone();
    env::set_var(env_var, graveyard);

    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            // We don't set the graveyard here!
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    assert!(!test_data.path.exists());
    assert!(test_env.graveyard.exists());

    let restored_data = fs::read_to_string(expected_graveyard_path).unwrap();
    assert_eq!(restored_data, test_data.data);

    restore_env_vars(default_env_vars);
}

#[rstest]
fn test_duplicate_file(
    #[values(false, true)] in_folder: bool,
    #[values(false, true)] inspect: bool,
) {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();

    // Bury the first file
    let test_data1 = if in_folder {
        fs::create_dir(test_env.src.join("dir")).unwrap();
        TestData::new(&test_env, Some("dir/file.txt"))
    } else {
        TestData::new(&test_env, Some("file.txt"))
    };
    let expected_graveyard_path1 =
        util::join_absolute(&test_env.graveyard, test_data1.path.canonicalize().unwrap());

    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [if in_folder {
                test_data1.path.parent().unwrap().to_path_buf()
            } else {
                test_data1.path.clone()
            }]
            .to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            inspect,
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    let log_s = String::from_utf8(log).unwrap();
    if inspect && in_folder {
        assert!(log_s.contains("dir: directory"));
        assert!(log_s.contains("including:"));
        assert!(log_s.contains("to the graveyard? (y/N)"));
    }

    assert!(expected_graveyard_path1.exists());

    // Bury the second file
    let test_data2 = if in_folder {
        // TODO: Why do we need to create the whole dir?
        fs::create_dir_all(test_env.src.join("dir")).unwrap();
        TestData::new(&test_env, Some("dir/file.txt"))
    } else {
        TestData::new(&test_env, Some("file.txt"))
    };

    let path_within_graveyard = (if in_folder {
        test_data2.path.parent().unwrap().to_path_buf()
    } else {
        test_data2.path.clone()
    })
    .canonicalize()
    .unwrap();

    let expected_graveyard_path2 = util::join_absolute(
        &test_env.graveyard,
        PathBuf::from(if in_folder {
            format!("{}~1/file.txt", path_within_graveyard.to_str().unwrap())
        } else {
            format!("{}~1", path_within_graveyard.to_str().unwrap())
        }),
    );

    let mut log = Vec::new();

    rip2::run(
        Args {
            targets: [if in_folder {
                test_data2.path.parent().unwrap().to_path_buf()
            } else {
                test_data2.path.clone()
            }]
            .to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // The second file will be in the same folder, but with '~1' appended
    assert!(expected_graveyard_path2.exists());

    // Navigate to the test_env.src directory
    let cur_dir = env::current_dir().unwrap();
    env::set_current_dir(&test_env.src).unwrap();
    let mut log = Vec::new();
    // Unbury using seance
    rip2::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            unbury: Some(Vec::new()),
            seance: true,
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // Now, both files should be restored, one with the original name and the other with '~1' appended
    assert!(test_data1.path.exists());
    if !in_folder {
        assert!(test_env.src.join("file.txt~1").exists());
    } else {
        assert!(test_env.src.join("dir~1/file.txt").exists());
    }
    env::set_current_dir(cur_dir).unwrap();
}

/// Test that big files trigger special behavior.
/// In this test, we simply delete it automatically.
#[rstest]
fn test_big_file() {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();
    // Access constant BIG_FILE_THRESHOLD from rip2's lib.rs:
    let size = rip2::BIG_FILE_THRESHOLD + 1;

    // test_env.src
    let big_file_path = test_env.src.join("big_file.txt");
    let file = fs::File::create(&big_file_path).unwrap();
    file.set_len(size).unwrap();

    let expected_graveyard_path =
        util::join_absolute(&test_env.graveyard, big_file_path.canonicalize().unwrap());

    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [test_env.src.join("big_file.txt")].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // The file should be deleted
    assert!(!test_env.src.join("big_file.txt").exists());

    // And not in the graveyard either
    assert!(!expected_graveyard_path.exists());
}

fn cli_runner<I, S>(args: I, cwd: Option<&PathBuf>) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
{
    let mut cmd = Command::cargo_bin("rip").unwrap();
    let mut cmd_ref = &mut cmd;
    cmd_ref.env_clear();
    if let Some(cwd) = cwd {
        cmd_ref.current_dir(cwd);
    }
    for arg in args {
        cmd_ref = cmd_ref.arg(arg);
    }
    cmd
}

fn quick_cmd_output(cmd: &mut Command) -> String {
    String::from_utf8(cmd.output().unwrap().stdout).unwrap()
}

/// Basic test of actually running the CLI itself
#[rstest]
fn test_cli(
    #[values(
        "help",
        "help2",
        "bury_unbury",
        "bury_seance",
        "bury_unbury_seance",
        "inspect",
        "inspect_no"
    )]
    scenario: &str,
) {
    let _env_lock = aquire_lock();
    let test_env = TestEnv::new();

    // Early exit for some tests
    if scenario.starts_with("help") {
        // Get output
        let mut cmd = match scenario {
            "help" => cli_runner(["--help"], None),
            "help2" => cli_runner(iter::empty::<&str>(), None),
            _ => unreachable!(),
        };
        let output = cmd.output().unwrap();
        assert!(output.status.success());
        let output_stdout = String::from_utf8(output.stdout).unwrap();
        assert!(output_stdout.contains("rip: a safe and ergonomic alternative to rm"));
        assert!(output_stdout.contains("Usage:"));
        assert!(output_stdout.contains("Options:"));
        return;
    }

    let base_args = vec!["--graveyard", test_env.graveyard.to_str().unwrap()];

    let names = ["test1.txt", "test2.txt", "dir/test.txt"];
    fs::create_dir_all(test_env.src.join("dir")).unwrap();

    names.map(|name| TestData::new(&test_env, Some(name)));
    // TODO: Check the data contents

    match scenario {
        scenario if scenario.starts_with("inspect") => {
            let mut args = base_args.clone();
            args.push("--inspect");
            args.push(names[0]);
            let mut cmd = cli_runner(args, Some(&test_env.src));
            match scenario {
                "inspect" => cmd.write_stdin("y"),
                "inspect_no" => cmd.write_stdin("n"),
                _ => unreachable!(),
            };

            let output = cmd.output().unwrap();
            let output_stdout = String::from_utf8(output.stdout).unwrap();

            assert!(
                output_stdout.contains(format!("{} to the graveyard? (y/N)", names[0]).as_str())
            );

            // One should still have the file, and the other should not:
            match scenario {
                "inspect" => assert!(!test_env.src.join(names[0]).exists()),
                "inspect_no" => assert!(test_env.src.join(names[0]).exists()),
                _ => unreachable!(),
            }
        }
        scenario if scenario.starts_with("bury") => {
            let mut bury_args = base_args.clone();
            bury_args.extend(names);
            let mut bury_cmd = cli_runner(&bury_args, Some(&test_env.src));
            let output_stdout = quick_cmd_output(&mut bury_cmd);
            assert!(output_stdout.is_empty());
            // Check only whitespace characters:
            assert!(output_stdout.chars().all(char::is_whitespace));

            let mut unbury_args = base_args.clone();

            if scenario.contains("unbury") {
                unbury_args.push("--unbury");
            }
            if scenario.contains("seance") {
                unbury_args.push("--seance");
            }
            let mut final_cmd = cli_runner(&unbury_args, Some(&test_env.src));
            let output_stdout = quick_cmd_output(&mut final_cmd);
            assert!(!output_stdout.is_empty());
            if scenario.contains("seance") {
                assert!(!names
                    .map(|name| {
                        let full_match = if scenario.contains("unbury") {
                            format!("{} to", name)
                        } else {
                            name.to_string()
                        };
                        output_stdout.contains(&full_match)
                    })
                    .iter()
                    .any(|has_name| !has_name));
            } else {
                // Only the last file should be unburied
                assert!(output_stdout.contains(names[2]));
                assert!(names
                    .map(|name| output_stdout.contains(name))
                    .iter()
                    .any(|has_name| !has_name));
            }
        }
        _ => unreachable!(),
    }
}
