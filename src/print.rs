/// print `[🟢Zeta]`
pub fn zeta_message(message: &str) {
    println!("[🟢Zeta] {}", message);
}

/// print `[🛑Zeta Error]`
pub fn zeta_error(message: &str) {
    println!("[🛑Zeta Error] {}", message);
}

/// print `[🛑Zeta Error]` with row and column position
pub fn zeta_error_position(message: &str, row: usize, column: usize) {
    zeta_error(format!("{}\n --> row: {}, column: {}", message, row, column).as_str());
}
