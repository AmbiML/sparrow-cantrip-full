// Arch-independent aliases.
pub type seL4_ASIDControl = seL4_ARM_ASIDControl;
pub type seL4_ASIDPool = seL4_ARM_ASIDPool;
pub type seL4_PageDirectory = seL4_ARM_PageDirectory;
pub type seL4_Page = seL4_ARM_Page;
pub type seL4_PageTable = seL4_ARM_PageTable;
pub type seL4_VMAttributes = seL4_ARM_VMAttributes;

pub use seL4_ObjectType::seL4_ARM_LargePageObject as seL4_LargePageObject;
pub use seL4_ObjectType::seL4_ARM_PageDirectoryObject as seL4_PageDirectoryObject;
pub use seL4_ObjectType::seL4_ARM_PageTableObject as seL4_PageTableObject;
pub use seL4_ObjectType::seL4_ARM_SmallPageObject as seL4_SmallPageObject;

pub use seL4_ARM_Default_VMAttributes as seL4_Default_VMAttributes;

pub use seL4_ARM_ASIDControl_MakePool as seL4_ASIDControl_MakePool;
pub use seL4_ARM_ASIDPool_Assign as seL4_ASIDPool_Assign;
pub use seL4_ARM_PageTable_Map as seL4_PageTable_Map;
pub use seL4_ARM_Page_GetAddress as seL4_Page_GetAddress;
// NB: seL4_Page_Map impl found below
pub use seL4_ARM_Page_Unmap as seL4_Page_Unmap;

pub unsafe fn seL4_Page_Map(
    sel4_page: seL4_CPtr,
    sel4_pd: seL4_CPtr,
    vaddr: seL4_Word,
    rights: seL4_CapRights,
    vm_attribs: seL4_VMAttributes,
) -> seL4_Result {
    if rights.get_capAllowGrant() != 0 {
        // NB: executable
        seL4_ARM_Page_Map(sel4_page, sel4_pd, vaddr, rights, vm_attribs)
    } else {
        seL4_ARM_Page_Map(sel4_page, sel4_pd, vaddr, rights, seL4_ARM_VMAttributes::ExecuteNever)
    }
}
