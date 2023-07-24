use anyhow::Result;
use colored::Colorize;
use fern::colors::{Color, ColoredLevelConfig};
use indoc::indoc;
use log::LevelFilter;

pub fn print_banner() {
    let banner = indoc! {
r#"

 _____   ___   _   _ ______  _____         ______  _____
/  ___| / _ \ | \ | ||  _  \|  _  |        | ___ \/  ___|
\ `--. / /_\ \|  \| || | | || | | | ______ | |_/ /\ `--.
 `--. \|  _  || . ` || | | || | | ||______||    /  `--. \
/\__/ /| | | || |\  || |/ / \ \_/ /        | |\ \ /\__/ /
\____/ \_| |_/\_| \_/|___/   \___/         \_| \_|\____/

______ __   __     _____       ___  ___ _____  _   _  _____  _____  _      _____  _____  _____
| ___ \\ \ / / _  |  _  |      |  \/  ||  _  || | | |/  ___||  ___|| |    |  ___|/  ___|/  ___|
| |_/ / \ V / (_) | |/' |__  __| .  . || | | || | | |\ `--. | |__  | |    | |__  \ `--. \ `--.
| ___ \  \ /      |  /| |\ \/ /| |\/| || | | || | | | `--. \|  __| | |    |  __|  `--. \ `--. \
| |_/ /  | |   _  \ |_/ / >  < | |  | |\ \_/ /| |_| |/\__/ /| |___ | |____| |___ /\__/ //\__/ /
\____/   \_/  (_)  \___/ /_/\_\\_|  |_/ \___/  \___/ \____/ \____/ \_____/\____/ \____/ \____/
"#};

    log::info!("{}", format!("{}", banner.green().bold()));
}

pub fn setup_logger() -> Result<()> {
    let colors = ColoredLevelConfig {
        trace: Color::Cyan,
        debug: Color::Magenta,
        info: Color::Green,
        warn: Color::Red,
        error: Color::BrightRed,
        ..ColoredLevelConfig::new()
    };

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .level(log::LevelFilter::Error)
        .level_for("rusty_sando", LevelFilter::Info)
        .level_for("strategy", LevelFilter::Info)
        .level_for("artemis_core", LevelFilter::Info)
        .apply()?;

    Ok(())
}
