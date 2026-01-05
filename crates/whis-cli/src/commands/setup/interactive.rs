//! Interactive prompt helpers using dialoguer
//!
//! Provides themed, consistent prompts for the setup wizard.

use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};

/// Get the shared theme for all prompts
pub fn theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

/// Select from a list of options with arrow keys
pub fn select<T: std::fmt::Display>(
    prompt: &str,
    items: &[T],
    default: Option<usize>,
) -> Result<usize> {
    let theme = theme();
    let mut select = Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(items);

    if let Some(idx) = default {
        select = select.default(idx);
    }

    Ok(select.interact()?)
}

/// Confirm yes/no with default
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    let theme = theme();
    Ok(Confirm::with_theme(&theme)
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

/// Get text input
pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    let theme = theme();
    let mut input = Input::with_theme(&theme).with_prompt(prompt);

    if let Some(d) = default {
        input = input.default(d.to_string());
    }

    Ok(input.interact_text()?)
}

/// Get password/secret input (hidden)
pub fn password(prompt: &str) -> Result<String> {
    let theme = theme();
    Ok(Password::with_theme(&theme)
        .with_prompt(prompt)
        .interact()?)
}

/// Print a styled header
pub fn header(text: &str) {
    println!();
    println!("{}", style(text).bold().cyan());
    println!();
}

/// Print a success message
pub fn success(text: &str) {
    println!("{} {}", style("✓").green().bold(), text);
}

/// Print an error message
pub fn error(text: &str) {
    eprintln!("{} {}", style("✗").red().bold(), text);
}

/// Print an info message
pub fn info(text: &str) {
    println!("{} {}", style("ℹ").blue(), text);
}
