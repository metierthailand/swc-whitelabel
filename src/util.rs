use crate::config::config;

pub fn report<F>(f: F) -> ()
where
    F: FnOnce() -> (),
{
    let is_allowed = !config::get().output_file_name_only;
    if is_allowed {
        f()
    };
}
