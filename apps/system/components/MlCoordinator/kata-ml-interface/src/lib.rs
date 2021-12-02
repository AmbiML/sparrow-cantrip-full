#![no_std]

pub trait MlCoordinatorInterface {
    fn execute(&mut self);
}

pub trait MlCoreInterface {
    fn enable_interrupts(&mut self, enabled: bool);
    fn clear_tcm(&mut self, start: *const u32, len: usize);
    fn run(&mut self);
    fn load_elf(&mut self, elf_slice: &[u8]) -> Result<(), &'static str>;
    fn get_return_code() -> u32;
    fn get_fault_register() -> u32;
    fn clear_host_req();
    fn clear_finish();
    fn clear_instruction_fault();
    fn clear_data_fault();
}
