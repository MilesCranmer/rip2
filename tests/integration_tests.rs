use lazy_static::lazy_static;
use predicates::str::is_match;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rip2::args::Args;
use rip2::record;
use rip2::util::TestMode;
use rip2::{self, util};
use rstest::rstest;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufReader, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::{env, ffi, iter};
use tempfile::{tempdir, TempDir};
use walkdir::WalkDir;

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
    fn new(test_env: &TestEnv, filename: Option<&PathBuf>) -> TestData {
        let data = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(100)
            .map(char::from)
            .collect::<String>();

        let path = if let Some(taken_filename) = filename {
            test_env.src.join(taken_filename)
        } else {
            test_env.src.join("test_file.txt")
        };
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
    let expected_graveyard_path = util::join_absolute(
        &test_env.graveyard,
        dunce::canonicalize(&test_data.path).unwrap(),
    );

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
        assert!(log_s.contains("100 B"));
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
    let expected_graveyard_path = util::join_absolute(
        modified_graveyard,
        dunce::canonicalize(&test_data.path).unwrap(),
    );

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
        TestData::new(&test_env, Some(&PathBuf::from("dir").join("file.txt")))
    } else {
        TestData::new(&test_env, Some(&PathBuf::from("file.txt")))
    };
    let expected_graveyard_path1 = util::join_absolute(
        &test_env.graveyard,
        dunce::canonicalize(&test_data1.path).unwrap(),
    );

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
        TestData::new(&test_env, Some(&PathBuf::from("dir").join("file.txt")))
    } else {
        TestData::new(&test_env, Some(&PathBuf::from("file.txt")))
    };

    let path_within_graveyard = dunce::canonicalize(if in_folder {
        test_data2.path.parent().unwrap().to_path_buf()
    } else {
        test_data2.path.clone()
    })
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
        assert!(
            test_env.src.join("file.txt~1").exists(),
            "Couldn't find file.txt~1 in {:?}",
            test_env.src
        );
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

    let expected_graveyard_path = util::join_absolute(
        &test_env.graveyard,
        dunce::canonicalize(big_file_path).unwrap(),
    );

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

/// Test that running rip on the same file twice
/// throws an error
#[rstest]
fn test_same_file_twice() {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env, None);

    let mut log = Vec::new();
    let result = rip2::run(
        Args {
            targets: [test_data.path.clone(), test_data.path.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    );

    // Check the first use triggered the removal:
    assert!(!test_data.path.exists());

    // Check the type of error
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);

    let err_msg = err.to_string();
    assert!(err_msg.contains("Cannot remove"));
    assert!(err_msg.contains("no such file or directory"));
}

fn cli_runner<I, S>(args: I, cwd: Option<&PathBuf>) -> assert_cmd::Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<ffi::OsStr>,
{
    let mut cmd = assert_cmd::Command::cargo_bin("rip").unwrap();
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for arg in args {
        cmd.arg(arg);
    }
    cmd.env("__RIP_ALLOW_RENAME", "false");
    cmd
}

fn quick_cmd_output(cmd: &mut assert_cmd::Command) -> String {
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

    fs::create_dir_all(test_env.src.join("dir")).unwrap();

    let paths = &[
        PathBuf::from("test1.txt"),
        PathBuf::from("test2.txt"),
        PathBuf::from("dir").join("test.txt"),
    ];
    let names = {
        let mut names = Vec::new();
        for name in paths {
            TestData::new(&test_env, Some(name));
            names.push(name.to_str().unwrap());
        }
        names
    };

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
            bury_args.extend(&names);
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
            assert!(
                !output_stdout.is_empty(),
                "Output was empty for scenario: {}",
                scenario
            );
            if scenario.contains("seance") {
                assert!(!names
                    .iter()
                    .map(|name| {
                        let full_match = if scenario.contains("unbury") {
                            format!("{} to", name)
                        } else {
                            name.to_string()
                        };
                        output_stdout.contains(&full_match)
                    })
                    .any(|has_name| !has_name));
            } else {
                // Only the last file should be unburied
                assert!(output_stdout.contains(names[2]));
                assert!(names
                    .iter()
                    .map(|name| output_stdout.contains(name))
                    .any(|has_name| !has_name));
            }
        }
        _ => unreachable!(),
    }
}

#[rstest]
fn issue_0018() {
    let _env_lock = aquire_lock();
    let test_env = TestEnv::new();

    // Make a big file
    {
        let size = rip2::BIG_FILE_THRESHOLD + 1;
        let file = fs::File::create(test_env.src.join("uu_meta.zip")).unwrap();
        file.set_len(size).unwrap();
    }

    // rip it and hit return to bury it anyways
    {
        let expected_graveyard_path = util::join_absolute(
            &test_env.graveyard,
            dunce::canonicalize(test_env.src.join("uu_meta.zip")).unwrap(),
        );
        cli_runner(
            [
                "--graveyard",
                test_env.graveyard.to_str().unwrap(),
                "uu_meta.zip",
            ],
            Some(&test_env.src),
        )
        .write_stdin("\n")
        .assert()
        .stdout(is_match("About to copy a big file").unwrap())
        .stdout(is_match("delete this file instead?").unwrap())
        .stdout(is_match("y/N").unwrap());

        // Expect it to be buried
        assert!(!test_env.src.join("uu_meta.zip").exists());
        assert!(expected_graveyard_path.exists());
    }

    // Make another big file
    {
        let size = rip2::BIG_FILE_THRESHOLD + 1;
        let file = fs::File::create(test_env.src.join("gnu_meta.zip")).unwrap();
        file.set_len(size).unwrap();
    }

    // rip it with interactive mode on, but quit
    {
        let expected_graveyard_path = util::join_absolute(
            &test_env.graveyard,
            dunce::canonicalize(test_env.src.join("gnu_meta.zip")).unwrap(),
        );
        cli_runner(
            [
                "--graveyard",
                test_env.graveyard.to_str().unwrap(),
                "-i",
                "gnu_meta.zip",
            ],
            Some(&test_env.src),
        )
        .write_stdin("q\n")
        .assert()
        .stdout(is_match("gnu_meta.zip: file, ").unwrap());

        // Expect it to remain in-place:
        assert!(test_env.src.join("gnu_meta.zip").exists());
        // And not in the graveyard:
        assert!(!expected_graveyard_path.exists());

        // The graveyard record should *only* reference uu_meta.zip:
        let record_contents = fs::read_to_string(test_env.graveyard.join(record::RECORD)).unwrap();
        assert!(record_contents.contains("uu_meta.zip"));
        assert!(!record_contents.contains("gnu_meta.zip"));

        // And give this for the last bury
        let record = record::Record::new(&test_env.graveyard);
        let last_bury = record.get_last_bury().unwrap();
        assert!(last_bury.ends_with("uu_meta.zip"));
    }

    // rip it again but without -i
    {
        // Should still be there
        assert!(test_env.src.join("gnu_meta.zip").exists());

        let expected_graveyard_path = util::join_absolute(
            &test_env.graveyard,
            dunce::canonicalize(test_env.src.join("gnu_meta.zip")).unwrap(),
        );

        cli_runner(
            [
                "--graveyard",
                test_env.graveyard.to_str().unwrap(),
                "gnu_meta.zip",
            ],
            Some(&test_env.src),
        )
        .write_stdin("y\n")
        .assert()
        .stdout(is_match("About to copy a big file").unwrap())
        .stdout(is_match("delete this file instead?").unwrap())
        .stdout(is_match("y/N").unwrap());

        // Expect it to be permanently deleted
        assert!(!test_env.src.join("gnu_meta.zip").exists());
        assert!(!expected_graveyard_path.exists());

        // The record should not reference it anymore either
        let record_contents = fs::read_to_string(test_env.graveyard.join(record::RECORD)).unwrap();
        assert!(!record_contents.contains("gnu_meta.zip"));
    }

    return;
}

#[rstest]
fn test_graveyard_subcommand(#[values(false, true)] seance: bool) {
    let _env_lock = aquire_lock();

    let expected_graveyard = rip2::get_graveyard(None);
    let cwd = &env::current_dir().unwrap();
    let expected_gravepath =
        util::join_absolute(&expected_graveyard, dunce::canonicalize(cwd).unwrap());
    let expected_str = if seance {
        format!("{}", expected_gravepath.display())
    } else {
        format!("{}", expected_graveyard.display())
    };
    let mut args = vec!["graveyard"];
    if seance {
        args.push("-s");
    }
    cli_runner(args, None)
        .assert()
        .success()
        .stdout(expected_str);
}

#[rstest]
fn read_empty_record() {
    let _env_lock = aquire_lock();
    let test_env = TestEnv::new();
    let cwd = env::current_dir().unwrap();
    fs::create_dir(&test_env.graveyard).unwrap();
    let record = record::Record::new(&test_env.graveyard);
    let gravepath = &util::join_absolute(&test_env.graveyard, dunce::canonicalize(cwd).unwrap());
    let result = record.seance(gravepath);
    assert!(result.is_ok());
}

/// Hash the directory and all contents
fn _hash_dir(dir: &PathBuf) -> String {
    let mut hash = DefaultHasher::new();
    for f in WalkDir::new(dir).sort_by(|a, b| a.cmp(b)) {
        let f = f.unwrap();
        let path = f.path();

        // First, hash the file path
        path.hash(&mut hash);
        if path.is_dir() {
            continue;
        }

        // Then, hash the file contents
        let file = fs::File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).unwrap();
        buffer.hash(&mut hash);
    }
    hash.finish().to_string()
}

/// Test that with many nested directories,
/// we can still bury and unbury files
#[rstest]
fn many_nest() {
    let test_env = TestEnv::new();

    // Random generators
    let pathname_len_range = 3..10;
    let depth_range = 1..5;
    let files_per_folder = 1..6;
    let bytes_range = 1..100;
    let num_folders = 50;

    // Inferred maximum number of files
    let max_num_files = (num_folders * (files_per_folder.end - 1) * (depth_range.end - 1)) as usize;

    // Vec of unique names to use
    let mut unique_rand_names = {
        let mut rand_names = Vec::new();
        while rand_names.len() < max_num_files {
            let dir_name_len = thread_rng().gen_range(pathname_len_range.clone());
            let rand_name = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(dir_name_len)
                .map(char::from)
                .collect::<String>();
            if !rand_names.contains(&rand_name) {
                rand_names.push(rand_name);
            }
        }
        rand_names
    };

    let depths = (0..num_folders).map(|_| thread_rng().gen_range(depth_range.clone()));
    let dirs = depths
        .map(|depth| {
            let mut path = test_env.src.clone();
            for _ in 0..depth {
                path = path.join(unique_rand_names.pop().unwrap());
            }
            path
        })
        .collect::<Vec<PathBuf>>();

    // Create the directories
    for dir in dirs.iter() {
        fs::create_dir_all(dir).unwrap();
    }

    // Create the filenames
    let filenames = {
        let mut filenames = Vec::new();
        for dir in dirs {
            let num_files = thread_rng().gen_range(files_per_folder.clone());
            for _ in 0..num_files {
                // Create an empty file
                let filename = dir.join(format!("{}.txt", unique_rand_names.pop().unwrap()));
                // Initialize the file
                filenames.push(filename);
            }
        }
        filenames
    };
    assert!(!filenames.is_empty());
    assert!(!unique_rand_names.is_empty());

    // Create the filenames with some data
    let num_bytes_per_file = filenames
        .iter()
        .map(|_| thread_rng().gen_range(bytes_range.clone()) as u64);
    let data = {
        let mut data = Vec::new();
        for (filename, num_bytes) in filenames.iter().zip(num_bytes_per_file) {
            // Create a file with `num_bytes` stored
            let mut file = fs::File::create(filename).unwrap();
            let cur_data = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(num_bytes as usize)
                .map(char::from)
                .collect::<String>();
            file.write_all(cur_data.as_bytes()).unwrap();
            data.push(cur_data);
        }
        data
    };

    // Check that the first file exists
    assert!(filenames[0].exists());

    // Check that it has the right data
    {
        let cur_data = fs::read_to_string(&filenames[0]).unwrap();
        assert_eq!(cur_data, data[0]);
    }

    // Get the true size
    let true_size = fs_extra::dir::get_size(&test_env.src).unwrap();

    // Hash everything in the directory
    let original_hash = _hash_dir(&test_env.src);

    // Bury the files interactively
    let mut log = Vec::new();
    let result = rip2::run(
        Args {
            targets: [test_env.src.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            inspect: true,
            ..Args::default()
        },
        TestMode,
        &mut log,
    );
    assert!(result.is_ok());
    let log_s = String::from_utf8(log).unwrap();
    let expected_log_s = format!(
        "{}: directory, {} including:",
        test_env.src.display(),
        util::humanize_bytes(true_size)
    );
    assert!(log_s.contains(&expected_log_s));

    // Unbury everything
    let mut log = Vec::new();
    let result = rip2::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            unbury: Some(Vec::new()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    );
    assert!(result.is_ok());

    // The hash should be unchanged
    let new_hash = _hash_dir(&test_env.src);
    assert_eq!(original_hash, new_hash);
}

#[rstest]
fn test_bury_unbury_bury_unbury() {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env, None);

    // First bury
    let expected_graveyard_path = util::join_absolute(
        &test_env.graveyard,
        dunce::canonicalize(&test_data.path).unwrap(),
    );

    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // Verify that the file is in the graveyard
    assert!(!test_data.path.exists());
    assert!(expected_graveyard_path.exists());

    // Get the record file's contents:
    let record_path = test_env.graveyard.join(record::RECORD);
    assert!(record_path.exists());
    let record_contents = fs::read_to_string(&record_path).unwrap();
    println!("Initial record contents:\n{}", record_contents);

    let normalized_test_data_path = dunce::canonicalize(&test_data.path).unwrap();
    assert!(record_contents.contains(&normalized_test_data_path.display().to_string()));

    // First unbury
    let mut log = Vec::new();
    rip2::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            unbury: Some(Vec::new()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // Verify that the file is restored
    assert!(test_data.path.exists());
    let restored_data = fs::read_to_string(&test_data.path).unwrap();
    assert_eq!(restored_data, test_data.data);

    // Get the new record file's contents:
    assert!(record_path.exists());
    let record_contents = fs::read_to_string(&record_path).unwrap();
    println!("After first unbury, record contents:\n{}", record_contents);

    // The record should still have the header:
    assert!(record_contents.contains("Time"));
    assert!(record_contents.contains("Original"));
    assert!(record_contents.contains("Destination"));

    // Second bury
    let mut log = Vec::new();
    rip2::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // Verify that the file is in the graveyard again
    assert!(!test_data.path.exists());
    assert!(expected_graveyard_path.exists());

    // Print the contents of the .record file
    let record_path = test_env.graveyard.join(record::RECORD);
    assert!(record_path.exists());

    // Make sure the record file contains the path
    let record_contents = fs::read_to_string(&record_path).unwrap();
    println!("Final record contents:\n{}", record_contents);

    let normalized_test_data_path = dunce::canonicalize(&test_data.path).unwrap();
    assert!(record_contents.contains(&normalized_test_data_path.display().to_string()));

    // Second unbury
    let mut log = Vec::new();
    rip2::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            unbury: Some(Vec::new()),
            ..Args::default()
        },
        TestMode,
        &mut log,
    )
    .unwrap();

    // Verify that the file is restored again
    assert!(test_data.path.exists());
    let restored_data = fs::read_to_string(&test_data.path).unwrap();
    assert_eq!(restored_data, test_data.data);
}
