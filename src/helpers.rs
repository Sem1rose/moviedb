pub fn ellipsize_string(string: &str, max_width: usize) -> String {
    let mut new_string = String::from(string);
    if new_string.len() > max_width {
        new_string.truncate(max_width - 3);
        new_string += "...";
    }

    new_string
}
