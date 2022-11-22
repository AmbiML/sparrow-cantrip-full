// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Arch-independent aliases.
pub type seL4_ASIDControl = seL4_X86_ASIDControl;
pub type seL4_ASIDPool = seL4_X86_ASIDPool;
pub type seL4_PageDirectory = seL4_X86_PageDirectory;
pub type seL4_Page = seL4_X86_Page;
pub type seL4_PageTable = seL4_X86_PageTable;
pub type seL4_VMAttributes = seL4_X86_VMAttributes;

pub use seL4_ObjectType::seL4_X86_LargePageObject as seL4_LargePageObject;
pub use seL4_ObjectType::seL4_X86_PageDirectoryObject as seL4_PageDirectoryObject;
pub use seL4_ObjectType::seL4_X86_PageTableObject as seL4_PageTableObject;
pub use seL4_ObjectType::seL4_X86_SmallPageObject as seL4_SmallPageObject;

pub use seL4_X86_Default_VMAttributes as seL4_Default_VMAttributes;

pub use seL4_X86_ASIDControl_MakePool as seL4_ASIDControl_MakePool;
pub use seL4_X86_ASIDPool_Assign as seL4_ASIDPool_Assign;
pub use seL4_X86_PageTable_Map as seL4_PageTable_Map;
pub use seL4_X86_Page_GetAddress as seL4_Page_GetAddress;
pub use seL4_X86_Page_Map as seL4_Page_Map;
pub use seL4_X86_Page_Unmap as seL4_Page_Unmap;
