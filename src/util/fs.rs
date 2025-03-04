use std::path::PathBuf;

pub fn combile_file_name_with_dir_name(old: String, new: String) -> String {
    let mut path = PathBuf::from(old);
    path.pop();
    path.push(new);
    path.to_string_lossy().into_owned()
}
