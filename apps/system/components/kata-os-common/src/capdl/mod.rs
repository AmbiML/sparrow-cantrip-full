// capDL specification support.
//
// This code started from a bindgen conversion of capdl.h; some vestiges
// of that are still present and could be improved.
//
// TODO(sleffler): test on non-riscv arch's
// TODO(sleffler): support for constructing specifications
#![no_std]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use core::mem::size_of;
use cstr_core;
use sel4_sys::seL4_CNode_CapData;
use sel4_sys::seL4_CPtr;
use sel4_sys::seL4_CapRights;
use sel4_sys::seL4_ObjectType;
use sel4_sys::seL4_ObjectTypeCount;
use sel4_sys::seL4_Word;
use sel4_sys::SEL4_BOOTINFO_HEADER_BOOTINFO;
use sel4_sys::SEL4_BOOTINFO_HEADER_FDT;
use sel4_sys::SEL4_BOOTINFO_HEADER_PADDING;
use sel4_sys::SEL4_BOOTINFO_HEADER_X86_ACPI_RSDP;
use sel4_sys::SEL4_BOOTINFO_HEADER_X86_FRAMEBUFFER;
use sel4_sys::SEL4_BOOTINFO_HEADER_X86_MBMMAP;
use sel4_sys::SEL4_BOOTINFO_HEADER_X86_TSC_FREQ;
use sel4_sys::SEL4_BOOTINFO_HEADER_X86_VBE;

use self::CDL_CapDataType::*;
use self::CDL_ObjectType::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct __BindgenBitfieldUnit<Storage> {
    storage: Storage,
}
impl<Storage> __BindgenBitfieldUnit<Storage> {
    #[inline]
    pub const fn new(storage: Storage) -> Self {
        Self { storage }
    }
}
impl<Storage> __BindgenBitfieldUnit<Storage>
where
    Storage: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    pub fn get_bit(&self, index: usize) -> bool {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = self.storage.as_ref()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        byte & mask == mask
    }
    #[inline]
    pub fn set_bit(&mut self, index: usize, val: bool) {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = &mut self.storage.as_mut()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        if val {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
    #[inline]
    pub fn get(&self, bit_offset: usize, bit_width: u8) -> u64 {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        let mut val = 0;
        for i in 0..(bit_width as usize) {
            if self.get_bit(i + bit_offset) {
                let index = if cfg!(target_endian = "big") {
                    bit_width as usize - 1 - i
                } else {
                    i
                };
                val |= 1 << index;
            }
        }
        val
    }
    #[inline]
    pub fn set(&mut self, bit_offset: usize, bit_width: u8, val: u64) {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        for i in 0..(bit_width as usize) {
            let mask = 1 << i;
            let val_bit_is_set = val & mask == mask;
            let index = if cfg!(target_endian = "big") {
                bit_width as usize - 1 - i
            } else {
                i
            };
            self.set_bit(index + bit_offset, val_bit_is_set);
        }
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
pub enum CDL_CapDataType {
    CDL_CapData_Badge,
    CDL_CapData_Guard,
    CDL_CapData_Raw,
}

// NB: we avoid an enum to simplify use
pub const CDL_TCB_CTable_Slot: seL4_Word = 0;
pub const CDL_TCB_VTable_Slot: seL4_Word = CDL_TCB_CTable_Slot + 1;
pub const CDL_TCB_Reply_Slot: seL4_Word = CDL_TCB_VTable_Slot + 1;
pub const CDL_TCB_Caller_Slot: seL4_Word = CDL_TCB_Reply_Slot + 1;
pub const CDL_TCB_IPCBuffer_Slot: seL4_Word = CDL_TCB_Caller_Slot + 1;
pub const CDL_TCB_FaultEP_Slot: seL4_Word = CDL_TCB_IPCBuffer_Slot + 1;
pub const CDL_TCB_SC_Slot: seL4_Word = CDL_TCB_FaultEP_Slot + 1;
pub const CDL_TCB_TemporalFaultEP_Slot: seL4_Word = CDL_TCB_SC_Slot + 1;
// CONFIG_ARM_HYPERVISOR_SUPPORT || CONFIG_VTX
pub const CDL_TCB_VCPU_Slot: seL4_Word = CDL_TCB_TemporalFaultEP_Slot + 1;

pub type CDL_ObjID = seL4_Word;
// NB: some object id's are written in the spec as -1
pub fn is_objid_valid(val: CDL_ObjID) -> bool {
    val != CDL_ObjID::MAX
}
pub type CDL_IRQ = seL4_Word;
pub type CDL_Core = seL4_Word;

#[repr(usize)]
pub enum CDL_CapRights {
    CDL_CanWrite = 1,      // BIT(0)
    CDL_CanRead = 2,       // BIT(1)
    CDL_CanGrant = 4,      // BIT(2)
    CDL_CanGrantReply = 8, // BIT(3)
    CDL_AllRights = 15,
}
impl From<CDL_CapRights> for seL4_CapRights {
    fn from(rights: CDL_CapRights) -> seL4_CapRights {
        // TODO(sleffler): simplify/cleanup ::new
        let val: usize = unsafe { ::core::mem::transmute(rights) };
        seL4_CapRights::new(
            ((val & 8) != 0) as usize,
            ((val & 4) != 0) as usize,
            ((val & 2) != 0) as usize,
            ((val & 1) != 0) as usize,
        )
    }
}

// NB: u8 required for use below
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CDL_CapType {
    CDL_NullCap = 0,
    CDL_UntypedCap,
    CDL_EPCap,
    CDL_NotificationCap,
    CDL_ReplyCap,
    CDL_MasterReplyCap,
    CDL_CNodeCap,
    CDL_TCBCap,
    CDL_IRQControlCap,
    CDL_IRQHandlerCap,
    CDL_FrameCap,
    CDL_PTCap,
    CDL_PDCap,
    CDL_PML4Cap,
    CDL_PDPTCap,
    CDL_PUDCap,
    CDL_PGDCap,
    CDL_ASIDControlCap,
    CDL_ASIDPoolCap,

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    CDL_IOPortsCap,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    CDL_IOSpaceCap,

    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    CDL_ARMIOSpaceCap,
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    CDL_SIDCap,
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    CDL_CBCap,

    CDL_SCCap,
    CDL_SchedControlCap,
    CDL_RTReplyCap,
    CDL_DomainCap,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CDL_CapData {
    pub __bindgen_anon: CDL_CapData__bindgen_ty_1,
    pub _bitfield_align: [u8; 0],
    pub _bitfield: __BindgenBitfieldUnit<[u8; 1]>,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union CDL_CapData__bindgen_ty_1 {
    pub __bindgen_anon: CDL_CapData__bindgen_ty_1__bindgen_ty_1,
    pub badge: seL4_Word,
    pub data: seL4_Word,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CDL_CapData__bindgen_ty_1__bindgen_ty_1 {
    pub _bitfield_align: [u32; 0],
    pub _bitfield: __BindgenBitfieldUnit<[u8; 4]>,
}
impl CDL_CapData__bindgen_ty_1__bindgen_ty_1 {
    #[inline]
    pub fn new(guard_bits: seL4_Word, guard_size: seL4_Word) -> Self {
        CDL_CapData__bindgen_ty_1__bindgen_ty_1 {
            _bitfield_align: [],
            _bitfield: Self::new_bitfield(guard_bits, guard_size),
        }
    }
    #[inline]
    pub fn guard_bits(&self) -> seL4_Word {
        self._bitfield.get(0, 18) as seL4_Word
    }
    #[inline]
    pub fn set_guard_bits(&mut self, val: seL4_Word) {
        self._bitfield.set(0, 18, val as u64)
    }
    #[inline]
    pub fn guard_size(&self) -> seL4_Word {
        self._bitfield.get(18, 14) as seL4_Word
    }
    #[inline]
    pub fn set_guard_size(&mut self, val: seL4_Word) {
        self._bitfield.set(18, 14, val as u64)
    }
    fn new_bitfield(
        guard_bits: seL4_Word,
        guard_size: seL4_Word,
    ) -> __BindgenBitfieldUnit<[u8; 4]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 4]> = Default::default();
        __bindgen_bitfield_unit.set(0, 18, guard_bits as u64);
        __bindgen_bitfield_unit.set(18, 14, guard_size as u64);
        __bindgen_bitfield_unit
    }
}
impl CDL_CapData {
    pub fn get_cap_data(&self) -> seL4_Word {
        match self.tag() {
            CDL_CapData_Badge => self.badge(),
            CDL_CapData_Guard => {
                seL4_CNode_CapData::new(self.guard_bits(), self.guard_size()).words[0]
            }
            CDL_CapData_Raw => self.data(),
        }
    }
    #[inline]
    pub fn tag(&self) -> CDL_CapDataType {
        unsafe { ::core::mem::transmute::<u32, CDL_CapDataType>(self._bitfield.get(0, 2) as u32) }
    }
    #[inline]
    pub fn set_tag(&mut self, val: u32) {
        self._bitfield.set(0, 2, val as u64)
    }
    #[inline]
    pub fn guard_bits(&self) -> seL4_Word {
        unsafe { self.__bindgen_anon.__bindgen_anon.guard_bits() }
    }
    #[inline]
    pub fn set_guard_bits(&mut self, val: seL4_Word) {
        unsafe { self.__bindgen_anon.__bindgen_anon.set_guard_bits(val) }
    }
    #[inline]
    pub fn guard_size(&self) -> seL4_Word {
        unsafe { self.__bindgen_anon.__bindgen_anon.guard_size() }
    }
    #[inline]
    pub fn set_guard_size(&mut self, val: seL4_Word) {
        unsafe { self.__bindgen_anon.__bindgen_anon.set_guard_size(val) }
    }
    #[inline]
    pub fn badge(&self) -> seL4_Word {
        unsafe { self.__bindgen_anon.badge }
    }
    #[inline]
    pub fn set_badge(&mut self, val: seL4_Word) {
        self.__bindgen_anon.badge = val
    }
    #[inline]
    pub fn data(&self) -> seL4_Word {
        unsafe { self.__bindgen_anon.data }
    }
    #[inline]
    pub fn set_data(&mut self, val: seL4_Word) {
        self.__bindgen_anon.data = val
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CDL_Cap {
    pub obj_id: CDL_ObjID,
    pub data: CDL_CapData,
    pub irq: CDL_IRQ,
    pub mapping_container_id: CDL_ObjID,
    pub mapping_slot: seL4_Word,
    pub mapped_frame_cap: seL4_CPtr,
    pub type_: CDL_CapType, // NB: ok to use enum 'cuz declared repr(C, u8)
    pub _bitfield_align: [u8; 0],
    pub _bitfield: __BindgenBitfieldUnit<[u8; 1]>,
}
impl CDL_Cap {
    // data in an seL4-friendly format
    pub fn cap_data(&self) -> seL4_Word {
        self.data.get_cap_data()
    }

    // Returns the sel4utils representation of a CDL_Cap's rights
    pub fn cap_rights(&self) -> seL4_CapRights {
        self.rights().into()
    }

    #[inline]
    pub fn r#type(&self) -> CDL_CapType {
        self.type_
    }
    #[inline]
    pub fn vm_attribs(&self) -> u32 {
        self._bitfield.get(0, 3) as u32
    }
    #[inline]
    pub fn set_vm_attribs(&mut self, val: u32) {
        self._bitfield.set(0, 3, val as u64)
    }
    #[inline]
    pub fn is_orig(&self) -> bool {
        self._bitfield.get(3, 1) != 0
    }
    #[inline]
    pub fn set_is_orig(&mut self, val: bool) {
        self._bitfield.set(3, 1, if val { 1u64 } else { 0u64 })
    }
    #[inline]
    pub fn rights(&self) -> CDL_CapRights {
        unsafe { ::core::mem::transmute::<usize, CDL_CapRights>(self._bitfield.get(4, 4) as usize) }
    }
    #[inline]
    pub fn set_rights(&mut self, val: CDL_CapRights) {
        unsafe {
            let val: usize = ::core::mem::transmute(val);
            self._bitfield.set(4, 4, val as u64)
        }
    }
    #[inline]
    fn new_bitfield(vm_attribs: u32, is_orig: u32, rights: u32) -> __BindgenBitfieldUnit<[u8; 1]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 1]> = Default::default();
        __bindgen_bitfield_unit.set(0, 3, vm_attribs as u64);
        __bindgen_bitfield_unit.set(3, 1, is_orig as u64);
        __bindgen_bitfield_unit.set(4, 4, rights as u64);
        __bindgen_bitfield_unit
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct CDL_CapSlot {
    pub slot: seL4_Word,
    pub cap: CDL_Cap,
}

#[repr(C)]
#[derive(Debug)]
pub struct CDL_CapMap {
    pub num: seL4_Word,
    pub slot: *const CDL_CapSlot,
}
impl<'a> CDL_CapMap {
    pub fn as_slice(&'a self) -> &'a [CDL_CapSlot] {
        unsafe { core::slice::from_raw_parts(self.slot, self.num) }
    }
    pub fn get_slot(&self, index: usize) -> CDL_CapSlot {
        self.as_slice()[index]
    }
    pub fn get_cap_at(&self, slot: seL4_Word) -> Option<&CDL_Cap> {
        self.as_slice()
            .iter()
            .find(|s| s.slot == slot)
            .map(|s| &s.cap)
    }
}

// Object tyype as found in the capdl input stream. This is partly defined
// in terms of the seL4_ObjectType and otherwise using seL4_LastObjectCount.
// The result is a mess and for zero gain (relative to just never reusing
// an enum member value).
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CDL_ObjectType {
    CDL_Endpoint = sel4_sys::seL4_EndpointObject as isize,
    CDL_Notification = sel4_sys::seL4_NotificationObject as isize,
    CDL_TCB = sel4_sys::seL4_TCBObject as isize,
    CDL_CNode = sel4_sys::seL4_CapTableObject as isize,
    CDL_Untyped = sel4_sys::seL4_UntypedObject as isize,

    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    CDL_SchedContext = sel4_sys::seL4_SchedContextObject as isize,
    #[cfg(feature = "CONFIG_KERNEL_MCS")]
    CDL_RTReply = sel4_sys::seL4_ReplyObject as isize,

    CDL_Frame = sel4_sys::seL4_SmallPageObject as isize,
    CDL_PT = sel4_sys::seL4_PageTableObject as isize,
    #[cfg(any(target_arch = "arm", target_arch = "aarch64", target_arch = "x86"))]
    CDL_PD = sel4_sys::seL4_PageDirectoryObject as isize,

    #[cfg(target_arch = "aarch64")]
    CDL_PUD = sel4_sys::seL4_PageUpperDirectoryObject as isize,
    #[cfg(target_arch = "aarch64")]
    CDL_PGD = sel4_sys::seL4_PageGlobalDirectoryObject as isize,

    #[cfg(any(feature = "CONFIG_ARM_HYPERVISOR_SUPPORT", feature = "CONFIG_VTX"))]
    CDL_VCPU = sel4_sys::seL4_VCPUObject as isize,

    #[cfg(target_arch = "x86_64")]
    CDL_PML4 = sel4_sys::seL4_PML4Object as isize,
    #[cfg(target_arch = "x86_64")]
    CDL_PDPT = sel4_sys::seL4_PDPTObject as isize,

    // NB: the following are numbered relative to seL4_ObjectTypeCount,
    //   do not re-order!
    CDL_ASIDPool = seL4_ObjectTypeCount + 1,
    CDL_Interrupt,
    CDL_IOPorts,  // CONFIG_ARCH_X86
    CDL_IODevice, // CONFIG_ARCH_X86

    // NB: when MCS is not enabled these are still defined (sigh)
    #[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
    CDL_SchedContext,
    #[cfg(not(feature = "CONFIG_KERNEL_MCS"))]
    CDL_RTReply,

    CDL_IOAPICInterrupt, // CONFIG_ARCH_X86
    CDL_MSIInterrupt,    // CONFIG_ARCH_X86
    CDL_ARMIODevice,     // CONFIG_ARCH_ARM
    CDL_PT_ROOT_ALIAS,   // NB: not used, placeholder
    CDL_ARMInterrupt,    // CONFIG_ARCH_ARM
    CDL_SID,             // CONFIG_ARCH_ARM
    CDL_CB,              // CONFIG_ARCH_ARM
}
impl From<CDL_ObjectType> for seL4_ObjectType {
    fn from(type_: CDL_ObjectType) -> seL4_ObjectType {
        // TODO(sleffler): maybe assert type_ < seL4_ObjectTypeCount
        unsafe { ::core::mem::transmute(type_) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_CNodeExtraInfo {
    pub has_untyped_memory: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_TCBExtraInfo {
    pub priority: u8,
    pub max_priority: u8,
    pub affinity: u8,
    pub domain: u8,
    pub pc: seL4_Word,
    pub sp: seL4_Word,
    pub elf_name: *const cstr_core::c_char,
    pub init: *const seL4_Word,
    pub init_sz: seL4_Word,
    pub fault_ep: seL4_CPtr,
    pub ipcbuffer_addr: seL4_Word,
    pub resume: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_SCExtraInfo {
    pub period: u64,
    pub budget: u64,
    pub data: seL4_Word,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_CBExtraInfo {
    pub bank: u8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_IOAPICIRQExtraInfo {
    pub ioapic: u32,
    pub ioapic_pin: u32,
    pub level: u32,
    pub polarity: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_MSIIRQExtraInfo {
    pub handle: u32,
    pub pci_bus: u32,
    pub pci_dev: u32,
    pub pci_fun: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_ARMIRQExtraInfo {
    pub trigger: u32,
    pub target: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CDL_FrameFillType_t {
    CDL_FrameFill_None = 0,
    CDL_FrameFill_BootInfo,
    CDL_FrameFill_FileData,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CDL_FrameFill_BootInfoEnum_t {
    // TODO(sleffler): C code defines Padding as FDT
    CDL_FrameFill_BootInfo_Padding = SEL4_BOOTINFO_HEADER_PADDING as isize,
    CDL_FrameFill_BootInfo_X86_VBE = SEL4_BOOTINFO_HEADER_X86_VBE as isize,
    CDL_FrameFill_BootInfo_X86_MBMMAP = SEL4_BOOTINFO_HEADER_X86_MBMMAP as isize,
    CDL_FrameFill_BootInfo_X86_ACPI_RSDP = SEL4_BOOTINFO_HEADER_X86_ACPI_RSDP as isize,
    CDL_FrameFill_BootInfo_X86_Framebuffer = SEL4_BOOTINFO_HEADER_X86_FRAMEBUFFER as isize,
    CDL_FrameFill_BootInfo_X86_TSC_Freq = SEL4_BOOTINFO_HEADER_X86_TSC_FREQ as isize,
    CDL_FrameFill_BootInfo_FDT = SEL4_BOOTINFO_HEADER_FDT as isize,
    CDL_FrameFill_BootInfo_BootInfo = SEL4_BOOTINFO_HEADER_BOOTINFO as isize,
}
impl From<CDL_FrameFill_BootInfoEnum_t> for usize {
    fn from(bi_type: CDL_FrameFill_BootInfoEnum_t) -> usize {
        bi_type as usize
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_FrameFill_BootInfoType_t {
    pub type_: CDL_FrameFill_BootInfoEnum_t,
    pub src_offset: usize,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_FrameFill_FileDataType_t {
    pub filename: *const cstr_core::c_char,
    pub file_offset: usize,
}
impl<'a> CDL_FrameFill_FileDataType_t {
    pub fn filename(&'a self) -> &'a str {
        unsafe { cstr_core::CStr::from_ptr(self.filename) }
            .to_str()
            .unwrap()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CDL_FrameFill_Element_t {
    pub type_: CDL_FrameFillType_t,
    pub dest_offset: usize,
    pub dest_len: usize,
    __bindgen_anon: CDL_FrameFill_Element_t__bindgen_ty_1,
}
impl<'a> CDL_FrameFill_Element_t {
    #[inline]
    pub fn get_bootinfo(&'a self) -> &'a CDL_FrameFill_BootInfoType_t {
        debug_assert!(self.type_ == CDL_FrameFillType_t::CDL_FrameFill_BootInfo);
        unsafe { &self.__bindgen_anon.bi_type }
    }
    #[inline]
    pub fn get_file_data(&'a self) -> &'a CDL_FrameFill_FileDataType_t {
        debug_assert!(self.type_ == CDL_FrameFillType_t::CDL_FrameFill_FileData);
        unsafe { &self.__bindgen_anon.file_data_type }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union CDL_FrameFill_Element_t__bindgen_ty_1 {
    pub bi_type: CDL_FrameFill_BootInfoType_t,
    pub file_data_type: CDL_FrameFill_FileDataType_t,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CDL_FrameExtraInfo {
    pub fill: [CDL_FrameFill_Element_t; 1],
    pub paddr: seL4_Word,
}

#[repr(C, packed)]
pub struct CDL_Object {
    #[cfg(feature = "CONFIG_DEBUG_BUILD")]
    pub name: *const cstr_core::c_char,
    pub slots: CDL_CapMap,
    pub extra: CDL_ObjectExtra,
    pub type_: CDL_ObjectType,
    pub size_bits: u32,
}
impl<'a> CDL_Object {
    #[cfg(feature = "CONFIG_DEBUG_BUILD")]
    pub fn name(&self) -> &str {
        if self.name.is_null() {
            "<null>"
        } else {
            unsafe { cstr_core::CStr::from_ptr(self.name) }
                .to_str()
                .unwrap()
        }
    }

    #[cfg(not(feature = "CONFIG_DEBUG_BUILD"))]
    #[inline]
    pub fn name(&self) -> &str {
        "<n/a>"
    }

    pub fn slots_slice(&'a self) -> &'a [CDL_CapSlot] {
        #[allow(unaligned_references)]
        self.slots.as_slice()
    }
    #[inline]
    pub fn num_slots(&self) -> seL4_Word {
        self.slots.num
    }
    // Returns the next available slot past those specified in the spec.
    // Note we cannot use num_slots since there may be gaps in the
    // numbering due to empty slots.
    #[inline]
    pub fn next_free_slot(&self) -> seL4_Word {
        self.slots_slice()
            .iter()
            .max_by_key(|slot| slot.slot)
            .map_or(0, |slot| slot.slot + 1)
    }
    #[inline]
    pub fn slot(&self, index: seL4_Word) -> CDL_CapSlot {
        #[allow(unaligned_references)]
        self.slots.get_slot(index)
    }
    #[inline]
    pub fn get_cap_at(&'a self, slot: seL4_Word) -> Option<&CDL_Cap> {
        #[allow(unaligned_references)]
        self.slots.get_cap_at(slot)
    }
    #[inline]
    pub fn r#type(&self) -> CDL_ObjectType {
        self.type_
    }
    #[inline]
    pub fn size_bits(&self) -> seL4_Word {
        self.size_bits as seL4_Word
    }
    pub fn paddr(&self) -> Option<seL4_Word> {
        match self.type_ {
            CDL_Frame => Some(unsafe { self.extra.frame_extra.paddr }),
            CDL_Untyped => Some(unsafe { self.extra.paddr }),
            _ => None,
        }
    }
    pub fn frame_fill(&'a self, index: usize) -> Option<&'a CDL_FrameFill_Element_t> {
        match self.type_ {
            CDL_Frame => Some(unsafe { &self.extra.frame_extra.fill[index] }),
            _ => None,
        }
    }
    pub fn is_device(&self) -> bool {
        // NB: must have a non-zero physical address
        self.paddr().map_or(false, |v| v != 0)
    }

    #[inline]
    pub fn cnode_has_untyped_memory(&self) -> bool {
        unsafe { self.extra.cnode_extra.has_untyped_memory }
    }

    // TODO(sleffler): maybe assert type_ before referencing union members
    // NB: return everything as seL4_Word to minimize conversions
    #[inline]
    pub fn tcb_ipcbuffer_addr(&self) -> seL4_Word {
        unsafe { self.extra.tcb_extra.ipcbuffer_addr }
    }
    #[inline]
    pub fn tcb_priority(&self) -> seL4_Word {
        (unsafe { self.extra.tcb_extra.priority }) as seL4_Word
    }
    #[inline]
    pub fn tcb_max_priority(&self) -> seL4_Word {
        (unsafe { self.extra.tcb_extra.max_priority }) as seL4_Word
    }
    #[inline]
    pub fn tcb_affinity(&self) -> seL4_Word {
        (unsafe { self.extra.tcb_extra.affinity }) as seL4_Word
    }
    #[inline]
    pub fn tcb_domain(&self) -> seL4_Word {
        (unsafe { self.extra.tcb_extra.domain }) as seL4_Word
    }
    #[inline]
    pub fn tcb_init(&self) -> *const seL4_Word {
        unsafe { self.extra.tcb_extra.init }
    }
    #[inline]
    pub fn tcb_init_sz(&self) -> seL4_Word {
        (unsafe { self.extra.tcb_extra.init_sz }) as seL4_Word
    }
    #[inline]
    pub fn tcb_pc(&self) -> seL4_Word {
        unsafe { self.extra.tcb_extra.pc }
    }
    #[inline]
    pub fn tcb_sp(&self) -> seL4_Word {
        unsafe { self.extra.tcb_extra.sp }
    }
    pub fn tcb_elf_name(&'a self) -> Option<&'a str> {
        unsafe {
            if self.extra.tcb_extra.elf_name.is_null() {
                None
            } else {
                cstr_core::CStr::from_ptr(self.extra.tcb_extra.elf_name)
                    .to_str()
                    .ok()
            }
        }
    }
    #[inline]
    pub fn tcb_resume(&self) -> bool {
        unsafe { self.extra.tcb_extra.resume }
    }
    #[inline]
    pub fn tcb_fault_ep(&self) -> seL4_CPtr {
        unsafe { self.extra.tcb_extra.fault_ep }
    }

    #[inline]
    pub fn cb_bank(&self) -> seL4_Word {
        (unsafe { self.extra.cb_extra.bank }) as seL4_Word
    }

    #[inline]
    pub fn sc_period(&self) -> u64 {
        unsafe { self.extra.sc_extra.period }
    }
    #[inline]
    pub fn sc_budget(&self) -> u64 {
        unsafe { self.extra.sc_extra.budget }
    }
    #[inline]
    pub fn sc_data(&self) -> seL4_Word {
        (unsafe { self.extra.sc_extra.data }) as seL4_Word
    }

    #[inline]
    pub fn other_start(&self) -> seL4_Word {
        (unsafe { self.extra.other.start }) as seL4_Word
    }
    #[inline]
    pub fn other_end(&self) -> seL4_Word {
        (unsafe { self.extra.other.end }) as seL4_Word
    }

    #[inline]
    pub fn msi_pci_bus(&self) -> seL4_Word {
        (unsafe { self.extra.msiirq_extra.pci_bus }) as seL4_Word
    }
    #[inline]
    pub fn msi_pci_dev(&self) -> seL4_Word {
        (unsafe { self.extra.msiirq_extra.pci_dev }) as seL4_Word
    }
    #[inline]
    pub fn msi_pci_fun(&self) -> seL4_Word {
        (unsafe { self.extra.msiirq_extra.pci_fun }) as seL4_Word
    }
    #[inline]
    pub fn msi_handle(&self) -> seL4_Word {
        (unsafe { self.extra.msiirq_extra.handle }) as seL4_Word
    }

    #[inline]
    pub fn ioapic_ioapic(&self) -> seL4_Word {
        (unsafe { self.extra.ioapicirq_extra.ioapic }) as seL4_Word
    }
    #[inline]
    pub fn ioapic_pin(&self) -> seL4_Word {
        (unsafe { self.extra.ioapicirq_extra.ioapic_pin }) as seL4_Word
    }
    #[inline]
    pub fn ioapic_level(&self) -> seL4_Word {
        (unsafe { self.extra.ioapicirq_extra.level }) as seL4_Word
    }
    #[inline]
    pub fn ioapic_polarity(&self) -> seL4_Word {
        (unsafe { self.extra.ioapicirq_extra.polarity }) as seL4_Word
    }

    #[inline]
    pub fn armirq_trigger(&self) -> seL4_Word {
        (unsafe { self.extra.armirq_extra.trigger }) as seL4_Word
    }
    #[inline]
    pub fn armirq_target(&self) -> seL4_Word {
        (unsafe { self.extra.armirq_extra.target }) as seL4_Word
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union CDL_ObjectExtra {
    pub tcb_extra: CDL_TCBExtraInfo,
    pub sc_extra: CDL_SCExtraInfo,
    pub cb_extra: CDL_CBExtraInfo,
    pub ioapicirq_extra: CDL_IOAPICIRQExtraInfo,
    pub msiirq_extra: CDL_MSIIRQExtraInfo,
    pub armirq_extra: CDL_ARMIRQExtraInfo,
    pub frame_extra: CDL_FrameExtraInfo,
    pub cnode_extra: CDL_CNodeExtraInfo,

    // Physical address; only defined for untyped objects.
    pub paddr: seL4_Word,

    // ASID pool assignment.
    pub _asid_high: seL4_Word,

    pub other: CDL_ObjectExtraOther,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_ObjectExtraOther {
    pub start: seL4_Word,
    pub end: seL4_Word,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_UntypedDerivation {
    pub untyped: CDL_ObjID,
    pub num: seL4_Word,
    pub children: *const CDL_ObjID,
}
impl CDL_UntypedDerivation {
    pub fn get_child(&self, index: usize) -> Option<CDL_ObjID> {
        if index >= self.num {
            return None;
        }
        Some(unsafe { ::core::slice::from_raw_parts(self.children, self.num) }[index])
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CDL_Model {
    // Object list.
    pub num: seL4_Word,
    pub objects: *const CDL_Object,

    // IRQ routing/assignments.
    pub num_irqs: seL4_Word,
    pub irqs: *const CDL_ObjID,

    // Untyped memory descriptors.
    pub num_untyped: seL4_Word,
    pub untyped: *const CDL_UntypedDerivation,

    // ASID slot number -> ASID pool object mapping.
    // NB: asid_slots[0] is unused because it is the rootserver's
    //     slot which is assigned by the kernel at boot.
    pub num_asid_slots: seL4_Word,
    pub asid_slots: *const CDL_ObjID,
}
impl<'a> CDL_Model {
    pub fn obj_slice(&'a self) -> &'a [CDL_Object] {
        unsafe { core::slice::from_raw_parts(self.objects, self.num) }
    }
    pub fn irq_slice(&'a self) -> &'a [CDL_ObjID] {
        unsafe { core::slice::from_raw_parts(self.irqs, self.num_irqs) }
    }
    pub fn untyped_slice(&'a self) -> &'a [CDL_UntypedDerivation] {
        unsafe { core::slice::from_raw_parts(self.untyped, self.num_untyped) }
    }
    pub fn asid_slot_slice(&'a self) -> &'a [CDL_ObjID] {
        unsafe { core::slice::from_raw_parts(self.asid_slots, self.num_asid_slots) }
    }

    // Calculate the space occupied by the capDL specification.
    pub fn calc_space(&self) -> usize {
        let mut total_space = size_of::<CDL_Model>()
            + self.num * size_of::<CDL_Object>()
            + self.num_irqs * size_of::<CDL_ObjID>()
            + self.num_untyped * size_of::<CDL_UntypedDerivation>()
            + self.num_asid_slots * size_of::<CDL_ObjID>();
        for obj in self.obj_slice() {
            #[cfg(feature = "CONFIG_DEBUG_BUILD")]
            if !obj.name.is_null() {
                total_space += obj.name().len() + 1;
            }
            total_space += obj.slots.num * size_of::<CDL_CapSlot>();
            match obj.r#type() {
                CDL_TCB => {
                    total_space += obj.tcb_init_sz() * size_of::<seL4_Word>();
                    if let Some(str) = obj.tcb_elf_name() {
                        total_space += str.len() + 1;
                    }
                }
                CDL_Frame => {
                    // TOOD(sleffler): iter over array instead of assuming 1
                    let frame_fill = obj.frame_fill(0).unwrap();
                    if frame_fill.type_ == CDL_FrameFillType_t::CDL_FrameFill_FileData {
                        total_space += frame_fill.get_file_data().filename().len() + 1;
                    }
                }
                _ => {}
            }
        }
        for &ut in self.untyped_slice() {
            total_space += ut.num * size_of::<CDL_ObjID>();
        }
        total_space
    }
}
