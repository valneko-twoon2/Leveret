//! Leveret's backend: the crate's modules, the `leveret --mcp` entry point and the
//! Tauri app setup (plugins, main window, registered commands).

mod agent;
mod capture;
mod commands;
mod crash;
mod difftool;
mod error;
mod git;
mod github;
mod issues;
mod lockdown;
mod perf;
mod repos;
mod security;
mod settings;
#[cfg(test)]
mod testutil;

use tauri::Manager;
use tauri::webview::PageLoadEvent;

use repos::RepoStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    perf::mark_process_start();
    crash::install_panic_hook();
    // `leveret --mcp`: serve the agent tools over stdio, without any window.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        std::process::exit(agent::mcp::serve());
    }
    let options = capture::parse_args(std::env::args());
    let capturing = options.offscreen();
    let page_loaded = std::sync::Once::new();

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(options)
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.manage(RepoStore::load(&settings::config_dir()));
            commands::app::create_main_window(app, capturing)?;
            if !capturing {
                commands::agent::watch_agent_activity(app.handle().clone());
            }
            perf::startup("window created");
            Ok(())
        })
        .on_page_load(move |webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                page_loaded.call_once(|| perf::startup("page loaded"));
                if !capturing {
                    let _ = webview.window().show();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::agent::agent_info,
            commands::agent::set_agent_access,
            commands::agent::agent_call,
            commands::app::startup_options,
            commands::app::capture_screen,
            commands::app::exit_app,
            commands::branches::list_branches,
            commands::branches::create_branch,
            commands::branches::switch_branch,
            commands::branches::checkout_commit,
            commands::branches::delete_branch,
            commands::changes::get_status,
            commands::changes::get_working_diff,
            commands::changes::commit,
            commands::changes::discard_changes,
            commands::changes::undo_last_commit,
            commands::changes::ignore_path,
            commands::changes::show_in_explorer,
            commands::difftool::get_diff_tool,
            commands::difftool::open_working_in_diff_tool,
            commands::difftool::open_commit_in_diff_tool,
            commands::clone::default_clone_dir,
            commands::clone::clone_repo,
            commands::github::github_state,
            commands::github::github_start_sign_in,
            commands::github::github_poll_sign_in,
            commands::github::github_sign_in_with_token,
            commands::github::github_sign_out,
            commands::github::github_list_repos,
            commands::github::open_github_url,
            commands::history::get_log,
            commands::history::get_commit,
            commands::history::get_commit_diff,
            commands::issues::issue_sources,
            commands::issues::list_issues,
            commands::issues::get_issue,
            commands::issues::create_issue,
            commands::issues::update_issue,
            commands::issues::comment_issue,
            commands::issues::delete_issue,
            commands::issues::publish_issue,
            commands::operations::revert_commit,
            commands::operations::rebase_branch,
            commands::operations::merge_branch,
            commands::operations::mark_resolved,
            commands::operations::continue_operation,
            commands::operations::skip_operation,
            commands::operations::abort_operation,
            commands::operations::cherry_pick,
            commands::perf::perf_samples,
            commands::perf::perf_report,
            commands::perf::bench_save,
            commands::remote::remote,
            commands::repos::list_repos,
            commands::repos::add_repo,
            commands::repos::remove_repo,
            commands::repos::select_repo,
            commands::stash::list_stashes,
            commands::stash::stash_save,
            commands::stash::stash_pop,
            commands::stash::stash_drop,
            commands::tags::list_tags,
            commands::tags::create_tag,
            commands::tags::delete_tag,
            commands::tags::push_tag,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        crash::fatal(&format!("Leveret could not start.\n\n{e}"));
    }
}
