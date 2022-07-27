// The Image Manager is responsible for loading and unloading multiple images
// into the Vector Core's tightly coupled memory. It tracks which image section
// is where and evicts images on the core when necessary.

// Design doc: go/sparrow-vc-memory.

// The memory is divided into "top" and "bottom" regions.
// The bottom region is shared between each model. It contains uninitialized
// values and the stack and heap. This space is shared in order to fit more
// models in memory. (This requires us to clear the memory between
// models from different applications.)
// The top region contains the sensor frames and the segments of each
// image. On system initialization the Sensor Manager requests an allocation,
// meaning the top of the memory will always contain those frames. Applications
// then request models to be loaded. These are allocated downward in a linear
// fashion. Images contain 6 different sections, of which 4 are loaded
// contiguously together (text, constant_data, model_output, static_data).
// All sections are described in go/sparrow-vc-memory.

// The expected most common usage patterns are:
// * There is only one model resident in memory.
// * There are two models resident in memory.
// * There are two models and each are too large to fit into memory together,
//   so they're unloaded and loaded on demand.
// Based on these expectations eviction is done on a FILO basis.

extern crate alloc;

use alloc::vec::Vec;
use core::cmp;
use cantrip_ml_shared::*;
use log::{info, trace};

#[cfg(test)]
use fake_vec_core as MlCore;
#[cfg(not(test))]
use cantrip_vec_core as MlCore;

// For each loaded image we need to track where the image's first segment is:
// data_top ---> +---------------+
//               |               |
//               | text          |
//               |               |
//               +---------------+
//               |               |
//               | constant_data |
//               |               |
//               +---------------+
//               |               |
//               | model_output  |
//               |               |
//               +---------------+
//               |               |
//               | static_data   |
//               |               |
//               +---------------+
// Each segment is page aligned.

#[derive(Debug)]
struct Image {
    id: ImageId,
    data_top_addr: usize,
    sizes: ImageSizes,
}

pub type ImageIdx = usize;

// NB: Can't use `None` as it's Option<T>, need to clarify its Option<Image>
const INIT_NONE: Option<Image> = None;

// ImageManager tracks three pointers into TCM:
//                   +---------------+
//                   |               |
//                   | sensor frames |
//                   |               |
// sensor_top -----> +---------------+
//                   |               |
//                   | model 1 data  |
//                   |               |
//                   +---------------+
//                   |               |
//                   | model 2 data  |
//                   |               |
// tcm_top    -----> +---------------+
//
//                    ..unused space..
//
// tcm_bottom -----> +---------------+
//                   |               |
//                   | shared temp   |
//                   |               |
//                   +---------------+
pub struct ImageManager {
    images: [Option<Image>; MAX_MODELS],
    image_queue: Vec<ImageIdx>,

    sensor_top: usize,
    tcm_top: usize,
    tcm_bottom: usize,
}

// Returns the bytes needed above current_size to fit requested_size.
fn space_needed(current_size: usize, requested_size: usize) -> usize {
    cmp::max(requested_size as isize - current_size as isize, 0) as usize
}

impl ImageManager {
    pub const fn new() -> Self {
        ImageManager {
            images: [INIT_NONE; MAX_MODELS],
            image_queue: Vec::new(),
            sensor_top: TCM_PADDR,
            tcm_top: TCM_PADDR,
            tcm_bottom: TCM_PADDR + TCM_SIZE,
        }
    }

    // Optional initilization step to reserve space for the queue.
    pub fn init(&mut self) { self.image_queue.reserve(MAX_MODELS); }

    // Allocate a block of memory for the SensorManager to use. Returns the
    // address of the block. This function should only be called once during
    // SensorManager initialization, before any images are loaded.
    pub fn allocate_sensor_input(&mut self, size: usize) -> usize {
        // Check no images have been loaded.
        assert_eq!(self.sensor_top, TCM_PADDR);
        let ret = self.sensor_top;
        self.sensor_top += round_up(size, WMMU_PAGE_SIZE);
        self.tcm_top = self.sensor_top;
        ret
    }

    fn tcm_top_size(&self) -> usize { self.tcm_top - TCM_PADDR }

    fn tcm_bottom_size(&self) -> usize { TCM_PADDR + TCM_SIZE - self.tcm_bottom }

    fn tcm_free_space(&self) -> usize { TCM_SIZE - self.tcm_top_size() - self.tcm_bottom_size() }

    // Returns the size of the largest temporary data block of loaded images.
    fn required_temporary_data(&self) -> usize {
        self.images
            .iter()
            .map(|opt| opt.as_ref().map_or(0, |image| image.sizes.temporary_data))
            .max()
            .map_or(0, |m| m)
    }

    // After images have been unloaded via unload_image we'll be left with
    // discontiguous spaces on the vector core. This function compacts the
    // images loaded in TCM to be contiguous.
    fn compact_tcm_top(&mut self) {
        let mut tcm_addr = self.sensor_top;

        for idx in &self.image_queue {
            if let Some(image) = self.images[*idx].as_mut() {
                let size = image.sizes.data_top_size();

                // Only move data if the addresses are different.
                if tcm_addr != image.data_top_addr {
                    MlCore::tcm_move(image.data_top_addr, tcm_addr, size);
                    image.data_top_addr = tcm_addr;
                }

                tcm_addr += size;
            }
        }
        self.tcm_top = tcm_addr;
    }

    // Removes the latest image loaded and return the size of the freed space.
    fn unload_latest(&mut self) -> ImageSizes {
        // We can assume there's an image in the queue and unwrap safely, as
        // otherwise we wouldn't need to unload images to fit new ones.
        let idx = self.image_queue.pop().unwrap();

        let (bundle, model) = self.ids_at(idx);
        info!("Unloading image {}:{}", bundle, model);

        self.images[idx].take().unwrap().sizes
    }

    /// Removes images in FILO order until the top TCM and temp TCM
    /// constraints are satisfied. Returns the address of the freed space.
    pub fn make_space(&mut self, top_tcm_needed: usize, temp_tcm_needed: usize) -> usize {
        assert!(top_tcm_needed + temp_tcm_needed <= TCM_SIZE);
        let mut available_tcm = self.tcm_free_space();
        let mut space_needed_for_temp =
            space_needed(self.required_temporary_data(), temp_tcm_needed);

        while available_tcm < top_tcm_needed + space_needed_for_temp {
            let freed_sizes = self.unload_latest();

            available_tcm += freed_sizes.data_top_size();

            self.tcm_top -= freed_sizes.data_top_size();

            // If we removed an image that had a temporary data size above the
            // current temp data size, we add that new memory to the pool.
            let remaining_temp = self.required_temporary_data();
            if freed_sizes.temporary_data > remaining_temp {
                available_tcm += freed_sizes.temporary_data - remaining_temp;
            }

            // Re-calculate space needed for temporary data given the new size.
            space_needed_for_temp = space_needed(remaining_temp, temp_tcm_needed);
        }

        self.tcm_top
    }

    // Sets the size of the temporary section based on the remaining images.
    fn set_tcm_bottom(&mut self) {
        let temp_data_size = self.required_temporary_data();
        self.tcm_bottom = TCM_PADDR + TCM_SIZE - temp_data_size;
        MlCore::set_wmmu_window(
            WindowId::TempData,
            self.tcm_bottom,
            temp_data_size,
            Permission::READ_WRITE,
        );
    }

    // Returns the index for image |id| if it exists.
    fn get_image_index(&self, id: &ImageId) -> Option<ImageIdx> {
        self.images.iter().position(|opti| {
            if let Some(i) = opti {
                i.id == *id
            } else {
                false
            }
        })
    }

    /// Returns true if the image |id| is currently loaded in the TCM.
    pub fn is_loaded(&mut self, id: &ImageId) -> bool { self.get_image_index(id).is_some() }

    /// Appends an (already written) image to internal book-keeping. This class
    /// does not handle the write as it requires seL4 references. The
    /// MlCoordinator must call this function after the write.
    pub fn commit_image(&mut self, id: ImageId, sizes: ImageSizes) {
        let image = Image {
            id,
            sizes,
            data_top_addr: self.tcm_top,
        };

        // We expect to always have <32 models due to memory constraints,
        // making this unwrap safe.
        let index = self.images.iter().position(|i| i.is_none()).unwrap();

        trace!("Adding image: {:x?}", image);

        self.image_queue.push(index);
        self.tcm_top += image.sizes.data_top_size();

        self.images[index] = Some(image);

        self.set_tcm_bottom();

        // If these pointers cross the memory is in an inconsistent state.
        // (We shouldn't hit this unless our space calculations are wrong.)
        assert!(self.tcm_bottom >= self.tcm_top);
    }

    // Unloads image |id| if loaded. Returns true if an image was unloaded.
    pub fn unload_image(&mut self, id: &ImageId) -> bool {
        if let Some(idx) = self.get_image_index(id) {
            self.image_queue.remove(idx);
            self.images[idx] = None;

            self.compact_tcm_top();
            self.set_tcm_bottom();
            return true;
        }

        false
    }

    /// Sets the WMMU to match the loaded image |id|. Returns true if that
    /// image exists and the WMMU was set.
    pub fn set_wmmu(&self, id: &ImageId) -> bool {
        if let Some(idx) = self.get_image_index(id) {
            let image = &self.images[idx].as_ref().unwrap();

            let mut top = image.data_top_addr;

            MlCore::set_wmmu_window(
                WindowId::Text,
                top,
                image.sizes.text,
                Permission::READ_EXECUTE,
            );
            top += image.sizes.text;

            MlCore::set_wmmu_window(
                WindowId::ConstData,
                top,
                image.sizes.constant_data,
                Permission::READ,
            );
            top += image.sizes.constant_data;

            MlCore::set_wmmu_window(
                WindowId::ModelOutput,
                top,
                image.sizes.model_output,
                Permission::READ_WRITE,
            );
            top += image.sizes.model_output;

            MlCore::set_wmmu_window(
                WindowId::StaticData,
                top,
                image.sizes.static_data,
                Permission::READ_WRITE,
            );

            // TODO(jesionowski): Set model_input window when sensor manager
            // is integrated.

            // NB: TEMP_DATA_WINDOW is set in set_tcm_bottom.

            return true;
        }

        false
    }

    /// Zeroes out the temporary data section.
    pub fn clear_temp_data(&self) { MlCore::clear_tcm(self.tcm_bottom, self.tcm_bottom_size()); }

    fn ids_at(&self, idx: ImageIdx) -> (&str, &str) {
        match self.images[idx].as_ref() {
            Some(image) => (&image.id.bundle_id, &image.id.model_id),
            None => ("None", "None"),
        }
    }

    /// Prints local state for debugging.
    pub fn debug_state(&self) {
        info!("Loaded Images:");
        for image in self.images.as_ref().iter().flatten() {
            info!("  {:x?}", image);
        }

        info!("Image Queue:");
        for idx in &self.image_queue {
            let (bundle, model) = self.ids_at(*idx);
            info!("  {}:{}", bundle, model);
        }

        info!("Sensor Top: 0x{:x}", self.sensor_top);
        info!("TCM Top: 0x{:x}", self.tcm_top);
        info!("TCM Bottom: 0x{:x}", self.tcm_bottom);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use alloc::string::ToString;
    use assert_hex::assert_eq_hex;

    #[test]
    fn allocate_sensor() {
        let mut image_manager = ImageManager::new();

        assert_eq_hex!(image_manager.allocate_sensor_input(0x1000), TCM_PADDR);

        assert_eq_hex!(image_manager.tcm_top_size(), 0x1000);
    }

    fn constant_image_size(size: usize) -> ImageSizes {
        ImageSizes {
            text: size,
            model_input: size,
            model_output: size,
            constant_data: size,
            static_data: size,
            temporary_data: size,
        }
    }

    fn make_id(model_id: usize) -> ImageId {
        ImageId {
            bundle_id: "B".to_string(),
            model_id: model_id.to_string(),
        }
    }

    fn default_id() -> ImageId { make_id(1) }

    fn load_image(image_manager: &mut ImageManager, id: ImageId, in_memory_sizes: ImageSizes) {
        image_manager.make_space(in_memory_sizes.data_top_size(), in_memory_sizes.temporary_data);

        image_manager.commit_image(id, in_memory_sizes);
    }

    // Load a model and see that is_loaded returns true. Unload and see false.
    #[test]
    fn load_unload() {
        let mut image_manager = ImageManager::new();
        load_image(&mut image_manager, default_id(), constant_image_size(0x1000));

        let id = default_id();

        assert!(image_manager.is_loaded(&id));

        assert!(image_manager.unload_image(&id));

        assert!(!image_manager.is_loaded(&id));
    }

    // Check that is_loaded returns false when an image isn't loaded, and
    // that unload_image returns false when attempting to load an image that
    // isn't loaded.
    #[test]
    fn is_loaded_no() {
        let mut image_manager = ImageManager::new();

        assert!(!image_manager.is_loaded(&default_id()));
        assert!(!image_manager.unload_image(&default_id()));
    }

    // Image that fills half the TCM. Zero temporary data in order to only
    // test tcm_top accounting.
    fn half_image() -> ImageSizes {
        ImageSizes {
            text: 0x20000,
            model_input: 0,
            model_output: 0x20000,
            constant_data: 0x20000,
            static_data: 0x20000,
            temporary_data: 0,
        }
    }

    // Image that fills all the TCM. Zero temporary data in order to only
    // test tcm_top accounting.
    fn full_image() -> ImageSizes {
        ImageSizes {
            text: 0x40000,
            model_input: 0,
            model_output: 0x40000,
            constant_data: 0x40000,
            static_data: 0x40000,
            temporary_data: 0,
        }
    }

    // Load two models that fit into the TCM and a third that forces an unload
    // of the second model. Then, load a 4th that unloads the others.
    #[test]
    fn loads_force_unloads() {
        let mut image_manager = ImageManager::new();

        let id1 = make_id(1);
        let id2 = make_id(2);
        let id3 = make_id(3);

        load_image(&mut image_manager, id1.clone(), half_image());
        load_image(&mut image_manager, id2.clone(), half_image());

        assert!(image_manager.is_loaded(&id1));
        assert!(image_manager.is_loaded(&id2));

        load_image(&mut image_manager, id3.clone(), half_image());

        assert!(image_manager.is_loaded(&id1));
        assert!(image_manager.is_loaded(&id3));

        let id4 = make_id(4);
        load_image(&mut image_manager, id4.clone(), full_image());
        assert!(image_manager.is_loaded(&id4));
    }

    // Load three models onto the TCM. Unload the second, and check that the
    // others have been compacted.
    #[test]
    fn unloads_compact_tcm() {
        let mut image_manager = ImageManager::new();

        let id1 = make_id(1);
        let id2 = make_id(2);
        let id3 = make_id(3);

        // Set different temporary data values.
        let sizes1 = ImageSizes {
            text: 0x1000,
            model_input: 0,
            model_output: 0x1000,
            constant_data: 0x1000,
            static_data: 0x1000,
            temporary_data: 0x2000,
        };

        let sizes2 = ImageSizes {
            text: 0x1000,
            model_input: 0,
            model_output: 0x1000,
            constant_data: 0x1000,
            static_data: 0x1000,
            temporary_data: 0x4000,
        };

        let sizes3 = ImageSizes {
            text: 0x1000,
            model_input: 0,
            model_output: 0x1000,
            constant_data: 0x1000,
            static_data: 0x1000,
            temporary_data: 0x3000, // This will be the largest post unload
        };

        load_image(&mut image_manager, id1.clone(), sizes1.clone());
        load_image(&mut image_manager, id2.clone(), sizes2.clone());
        load_image(&mut image_manager, id3.clone(), sizes3.clone());

        // The third image will be available at images[2]. Before unloading we
        // validate that it's past the first two models.
        assert_eq_hex!(
            image_manager.images[2].as_ref().unwrap().data_top_addr,
            TCM_PADDR + sizes1.data_top_size() + sizes2.data_top_size()
        );

        assert!(image_manager.unload_image(&id2));

        // After compacting it should be past just the first model.
        assert_eq_hex!(
            image_manager.images[2].as_ref().unwrap().data_top_addr,
            TCM_PADDR + sizes1.data_top_size()
        );

        assert_eq_hex!(
            image_manager.tcm_top_size(),
            sizes1.data_top_size() + sizes3.data_top_size()
        );
        assert_eq_hex!(image_manager.tcm_bottom_size(), 0x3000);
    }
}
