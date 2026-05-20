use console::Style;

pub fn print_error(msg: &str) {
    let red = Style::new().red().bold();
    eprintln!("{} {}", red.apply_to("ERROR:"), msg);
}

pub fn print_success(msg: &str) {
    let green = Style::new().green().bold();
    println!("{} {}", green.apply_to("✓"), msg);
}

pub fn print_info(msg: &str) {
    let blue = Style::new().cyan().bold();
    println!("{} {}", blue.apply_to("ℹ"), msg);
}

pub fn print_tip(msg: &str) {
    let yellow = Style::new().yellow().bold();
    println!("{} {}", yellow.apply_to("💡"), msg);
}

pub fn print_warning(msg: &str) {
    let yellow = Style::new().yellow().bold();
    println!("{} {}", yellow.apply_to("⚠"), msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_error_does_not_panic() {
        print_error("test error message");
    }

    #[test]
    fn test_print_success_does_not_panic() {
        print_success("test success message");
    }

    #[test]
    fn test_print_info_does_not_panic() {
        print_info("test info message");
    }

    #[test]
    fn test_print_tip_does_not_panic() {
        print_tip("test tip message");
    }

    #[test]
    fn test_print_error_empty_message() {
        print_error("");
    }

    #[test]
    fn test_print_success_empty_message() {
        print_success("");
    }
}
