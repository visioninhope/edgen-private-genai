use std::fs::remove_dir_all;
use std::path;

use reqwest::blocking;

use edgen_core::settings;
use edgen_server::status;

mod common;

#[test]
// The fake test serves for trying simple things out
fn fake_test() {
    println!("hello fake!");
}

#[test]
// Be aware that this test is long-running and downloads several GiB from huggingface.
// Since it tests the edgen environment (config and model directories),
// a backup of the user environment is created before the tests start
// and restored when the tests have passed (or failed).
// Should backup or restore fail, parts of your environment may have moved away.
// You find them in 'crates/edgen_server/env_backup' and may want to restore them manually.
// Run with
// cargo test --test settings_tests -- --show-output --nocapture
// (without --nocapture the output is shown only after finishing)
//
// Currently five scenarios are exercised (please update if you add new test scenarios):
// - SCENARIO 1:
//   + Start edgen server without files, they shall exist afterwards
//   + Endpoints shall be reachable
// - SCENARIO 2:
//   + Edit configuration: change should be reflected in server.
//   + AI Endpoints shall be reachable
//   + Model files shall be downloaded
//   + Once downloaded, no download is preformed again
// - SCENARIO 3:
//   + Change model directories: make sure now directories are created
//   + Model files shall be downloaded
//   + Once downloaded, no download is preformed again
// - SCENARIO 4:
//   + Remove model directories: make sure they are created again
//   + Model files shall be downloaded
//   + Once downloaded, no download is preformed again
// - SCENARIO 5:
//   + Reset configuration
//   + Model files shall not be downloaded
fn test_battery() {
    common::with_save_edgen(|| {
        // make sure everything is right
        pass_always();

        // ================================
        common::test_message("SCENARIO 1");
        // ================================
        config_exists();
        data_exists();

        // endpoints reachable
        connect_to_server_test();

        chat_completions_status_reachable();
        audio_transcriptions_status_reachable();

        // ================================
        common::test_message("SCENARIO 2");
        // ================================
        // set small models, so we don't need to download too much
        set_model(
            common::Endpoint::ChatCompletions,
            common::SMALL_LLM_NAME,
            common::SMALL_LLM_REPO,
        );
        set_model(
            common::Endpoint::AudioTranscriptions,
            common::SMALL_WHISPER_NAME,
            common::SMALL_WHISPER_REPO,
        );

        // test ai endpoint and download
        test_ai_endpoint_with_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_with_download(common::Endpoint::AudioTranscriptions);

        // we have downloaded, we should not download again
        test_ai_endpoint_no_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_no_download(common::Endpoint::AudioTranscriptions);

        // ================================
        common::test_message("SCENARIO 3");
        // ================================
        let my_models_dir = format!(
            "{}{}{}",
            common::BACKUP_DIR,
            path::MAIN_SEPARATOR,
            common::MY_MODEL_FILES,
        );

        let new_chat_completions_dir = my_models_dir.clone()
            + &format!(
                "{}{}{}{}",
                path::MAIN_SEPARATOR,
                "chat",
                path::MAIN_SEPARATOR,
                "completions",
            );

        let new_audio_transcriptions_dir = my_models_dir.clone()
            + &format!(
                "{}{}{}{}",
                path::MAIN_SEPARATOR,
                "audio",
                path::MAIN_SEPARATOR,
                "transcriptions",
            );

        set_model_dir(common::Endpoint::ChatCompletions, &new_chat_completions_dir);

        set_model_dir(
            common::Endpoint::AudioTranscriptions,
            &new_audio_transcriptions_dir,
        );

        test_ai_endpoint_with_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_with_download(common::Endpoint::AudioTranscriptions);

        assert!(path::Path::new(&my_models_dir).exists());

        test_ai_endpoint_no_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_no_download(common::Endpoint::AudioTranscriptions);

        // ================================
        common::test_message("SCENARIO 4");
        // ================================
        remove_dir_all(&my_models_dir).unwrap();
        assert!(!path::Path::new(&my_models_dir).exists());

        test_ai_endpoint_with_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_with_download(common::Endpoint::AudioTranscriptions);

        assert!(path::Path::new(&my_models_dir).exists());

        test_ai_endpoint_no_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_no_download(common::Endpoint::AudioTranscriptions);

        // ================================
        common::test_message("SCENARIO 5");
        // ================================
        test_config_reset();

        set_model(
            common::Endpoint::ChatCompletions,
            common::SMALL_LLM_NAME,
            common::SMALL_LLM_REPO,
        );
        set_model(
            common::Endpoint::AudioTranscriptions,
            common::SMALL_WHISPER_NAME,
            common::SMALL_WHISPER_REPO,
        );

        // make sure we read from the old directories again
        remove_dir_all(&my_models_dir).unwrap();
        assert!(!path::Path::new(&my_models_dir).exists());

        test_ai_endpoint_no_download(common::Endpoint::ChatCompletions);
        test_ai_endpoint_no_download(common::Endpoint::AudioTranscriptions);
    })
}

fn pass_always() {
    common::test_message("pass always");
    assert!(true);
}

// exercise the edgen version endpoint to make sure the server is reachable.
fn connect_to_server_test() {
    common::test_message("connect to server");
    assert!(match blocking::get(common::make_url(&[
        common::BASE_URL,
        common::MISC_URL,
        common::VERSION_URL
    ])) {
        Err(e) => {
            eprintln!("cannot connect: {:?}", e);
            false
        }
        Ok(v) => {
            println!("have: '{}'", v.text().unwrap());
            true
        }
    });
}

fn config_exists() {
    common::test_message("config exists");
    assert!(settings::PROJECT_DIRS.config_dir().exists());
    assert!(settings::CONFIG_FILE.exists());
}

fn data_exists() {
    common::test_message("data exists");
    let data = settings::PROJECT_DIRS.data_dir();
    println!("exists: {:?}", data);
    assert!(data.exists());

    let models = data.join("models");
    println!("exists: {:?}", models);
    assert!(models.exists());

    let chat = models.join("chat");
    println!("exists: {:?}", chat);
    assert!(models.exists());

    let completions = chat.join("completions");
    println!("exists: {:?}", completions);
    assert!(completions.exists());

    let audio = models.join("audio");
    println!("exists: {:?}", audio);
    assert!(audio.exists());

    let transcriptions = audio.join("transcriptions");
    println!("exists: {:?}", transcriptions);
    assert!(transcriptions.exists());
}

fn chat_completions_status_reachable() {
    common::test_message("chat completions status is reachable");
    assert!(match blocking::get(common::make_url(&[
        common::BASE_URL,
        common::CHAT_URL,
        common::COMPLETIONS_URL,
        common::STATUS_URL,
    ])) {
        Err(e) => {
            eprintln!("cannot connect: {:?}", e);
            false
        }
        Ok(v) => {
            println!("have: '{}'", v.text().unwrap());
            true
        }
    });
}

fn audio_transcriptions_status_reachable() {
    common::test_message("audio transcriptions status is reachable");
    assert!(match blocking::get(common::make_url(&[
        common::BASE_URL,
        common::AUDIO_URL,
        common::TRANSCRIPTIONS_URL,
        common::STATUS_URL,
    ])) {
        Err(e) => {
            eprintln!("cannot connect: {:?}", e);
            false
        }
        Ok(v) => {
            println!("have: '{}'", v.text().unwrap());
            true
        }
    });
}

// edit the config file: set another model name and repo for the indicated endpoint.
fn set_model(ep: common::Endpoint, model_name: &str, model_repo: &str) {
    common::test_message(&format!("set {} model to {}", ep, model_name,));

    let mut config = common::get_config().unwrap();

    match &ep {
        common::Endpoint::ChatCompletions => {
            config.chat_completions_model_name = model_name.to_string();
            config.chat_completions_model_repo = model_repo.to_string();
        }
        common::Endpoint::AudioTranscriptions => {
            config.audio_transcriptions_model_name = model_name.to_string();
            config.audio_transcriptions_model_repo = model_repo.to_string();
        }
    }
    common::write_config(&config).unwrap();

    println!("pausing for 4 secs to make sure the config file has been updated");
    std::thread::sleep(std::time::Duration::from_secs(4));
    let url = match ep {
        common::Endpoint::ChatCompletions => common::make_url(&[
            common::BASE_URL,
            common::CHAT_URL,
            common::COMPLETIONS_URL,
            common::STATUS_URL,
        ]),
        common::Endpoint::AudioTranscriptions => common::make_url(&[
            common::BASE_URL,
            common::AUDIO_URL,
            common::TRANSCRIPTIONS_URL,
            common::STATUS_URL,
        ]),
    };
    let stat: status::AIStatus = blocking::get(url).unwrap().json().unwrap();
    assert_eq!(stat.active_model, model_name);
}

// edit the config file: set another model dir for the indicated endpoint.
fn set_model_dir(ep: common::Endpoint, model_dir: &str) {
    common::test_message(&format!("set {} model directory to {}", ep, model_dir,));

    let mut config = common::get_config().unwrap();

    match &ep {
        common::Endpoint::ChatCompletions => {
            config.chat_completions_models_dir = model_dir.to_string();
        }
        common::Endpoint::AudioTranscriptions => {
            config.audio_transcriptions_models_dir = model_dir.to_string();
        }
    }
    common::write_config(&config).unwrap();

    println!("pausing for 4 secs to make sure the config file has been updated");
    std::thread::sleep(std::time::Duration::from_secs(4));
}

fn test_config_reset() {
    common::test_message("test resetting config");
    common::reset_config();

    println!("pausing for 4 secs to make sure the config file has been updated");
    std::thread::sleep(std::time::Duration::from_secs(4));
}

fn test_ai_endpoint_with_download(endpoint: common::Endpoint) {
    test_ai_endpoint(endpoint, true);
}

fn test_ai_endpoint_no_download(endpoint: common::Endpoint) {
    test_ai_endpoint(endpoint, false);
}

fn test_ai_endpoint(endpoint: common::Endpoint, download: bool) {
    let (statep, body) = match endpoint {
        common::Endpoint::ChatCompletions => {
            common::test_message(&format!(
                "chat completions endpoint with download: {}",
                download
            ));
            (
                common::make_url(&[
                    common::BASE_URL,
                    common::CHAT_URL,
                    common::COMPLETIONS_URL,
                    common::STATUS_URL,
                ]),
                common::CHAT_COMPLETIONS_BODY.to_string(),
            )
        }
        common::Endpoint::AudioTranscriptions => {
            common::test_message(&format!(
                "audio transcriptions endpoint with download: {}",
                download
            ));
            (
                common::make_url(&[
                    common::BASE_URL,
                    common::AUDIO_URL,
                    common::TRANSCRIPTIONS_URL,
                    common::STATUS_URL,
                ]),
                "".to_string(),
            )
        }
    };
    let handle = common::spawn_request(endpoint, body);
    if download {
        common::assert_download(&statep);
    } else {
        common::assert_no_download(&statep);
    }
    assert!(handle.join().unwrap());
}
