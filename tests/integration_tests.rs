use rand::distributions::Alphanumeric;
use rand::Rng;
use rip::args::Args;
use rip::{self, util};
use rstest::rstest;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use tempfile::{tempdir, TempDir};

use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_LOCK: Mutex<()> = Mutex::new(());
}

fn aquire_lock() -> MutexGuard<'static, ()> {
    GLOBAL_LOCK.lock().unwrap()
}

struct TestMode;
impl util::TestingMode for TestMode {
    fn is_test(&self) -> bool {
        true
    }
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
        let mut file = File::create(&path).unwrap();
        file.write_all(data.as_bytes()).unwrap();

        TestData { data, path }
    }
}

/// Test that a file is buried and unburied correctly
/// Also checks that the graveyard is deleted when decompose is true
#[rstest]
#[case(false, false)]
#[case::with_inspect(false, true)]
#[case::decomposition(true, false)]
fn test_bury_unbury(#[case] decompose: bool, #[case] inspect: bool) {
    let _env_lock = aquire_lock();

    let test_env = TestEnv::new();
    let test_data = TestData::new(&test_env, None);
    // And is now in the graveyard
    let expected_graveyard_path =
        util::join_absolute(&test_env.graveyard, test_data.path.canonicalize().unwrap());

    rip::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            inspect,
            ..Args::default()
        },
        TestMode,
    )
    .unwrap();

    // Verify that the file no longer exists
    assert!(!test_data.path.exists());

    // Verify that the graveyard exists
    assert!(test_env.graveyard.exists());
    assert!(expected_graveyard_path.exists());

    // with the right data
    let restored_data_from_grave = fs::read_to_string(&expected_graveyard_path).unwrap();
    assert_eq!(restored_data_from_grave, test_data.data);

    rip::run(
        Args {
            graveyard: Some(test_env.graveyard.clone()),
            decompose,
            unbury: if decompose { None } else { Some(Vec::new()) },
            ..Args::default()
        },
        TestMode,
    )
    .unwrap();

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

const ENV_VARS: [&str; 2] = ["GRAVEYARD", "XDG_DATA_HOME"];

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
#[case::graveyard("GRAVEYARD")]
#[case::xdg_data_home("XDG_DATA_HOME")]
fn test_env(#[case] env_var: &str) {
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

    rip::run(
        Args {
            targets: [test_data.path.clone()].to_vec(),
            // We don't set the graveyard here!
            ..Args::default()
        },
        TestMode,
    )
    .unwrap();

    assert!(!test_data.path.exists());
    assert!(test_env.graveyard.exists());

    let restored_data = fs::read_to_string(expected_graveyard_path).unwrap();
    assert_eq!(restored_data, test_data.data);

    restore_env_vars(default_env_vars);
}

#[rstest]
#[case(false)]
#[case::within_folder(true)]
fn test_duplicate_file(#[case] in_folder: bool) {
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

    rip::run(
        Args {
            targets: [if in_folder {
                test_data1.path.parent().unwrap().to_path_buf()
            } else {
                test_data1.path.clone()
            }]
            .to_vec(),
            graveyard: Some(test_env.graveyard.clone()),
            ..Args::default()
        },
        TestMode,
    )
    .unwrap();

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

    rip::run(
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
    )
    .unwrap();

    // The second file will be in the same folder, but with '~1' appended
    assert!(expected_graveyard_path2.exists());
}
