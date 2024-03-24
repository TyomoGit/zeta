pub fn zeta_message(message: &str) {
    println!("[🟢Zeta] {}", message);
}

pub fn zeta_error(message: &str) {
    println!("[🛑Zeta Error] {}", message);
}

pub fn zeta_error_position(message: &str, row: usize, column: usize) {
    zeta_error(format!("{}\n --> row: {}, column: {}", message, row, column).as_str());
}
