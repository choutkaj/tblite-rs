#![cfg(feature = "abi-tests")]
unsafe extern "C" {
    fn tblite_rs_config_size() -> usize;
    fn tblite_rs_config_align() -> usize;
    fn tblite_rs_link_symbols() -> usize;
}
#[test]
fn installed_c_abi_and_all_symbols() {
    unsafe {
        assert_eq!(
            tblite_rs_config_size(),
            std::mem::size_of::<tblite_sys::tblite_xtb_config>()
        );
        assert_eq!(
            tblite_rs_config_align(),
            std::mem::align_of::<tblite_sys::tblite_xtb_config>()
        );
        assert_eq!(tblite_rs_link_symbols(), 117);
        assert!((700..800).contains(&tblite_sys::tblite_get_version()));
    }
}
