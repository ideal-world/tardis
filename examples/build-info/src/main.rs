pub fn main() {
    const GIT_VERSION: &str = tardis::utils::build_info::git_version!();
    const PKG_VERSION: &str = tardis::pkg_version!();
    const PKG_NAME: &str = tardis::pkg!();
    println!("{} {} {}", PKG_NAME, PKG_VERSION, GIT_VERSION);
}
