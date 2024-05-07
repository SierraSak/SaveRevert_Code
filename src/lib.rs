#![feature(lazy_cell, ptr_sub_ptr)]
use std::cmp::Ordering;
use unity::prelude::*;
use engage::{
    stream::Stream,
    gamedata::{
        accessory::AccessoryData, unit::{
            UnitAccessory,
            UnitAccessoryList
        }, Gamedata
    },
};

use skyline::{
    hook,
    hooks::InlineCtx,
};

#[unity::hook("App", "UnitAccessoryList", "get_Count")]
pub fn unitaccessorylist_get_count(_this: &mut UnitAccessoryList, _method_info: OptionalMethod) -> i32 {
    return 8;
}

#[unity::hook("App", "UnitAccessoryList", ".ctor")]
pub fn unitaccessorylist_ctor_hook(this: &mut UnitAccessoryList, method_info: OptionalMethod,)
{
    call_original!(this, method_info);

    // Il2CppArray can be turned into a slice (https://doc.rust-lang.org/std/primitive.slice.html) and slices can be iterated (https://doc.rust-lang.org/std/iter/trait.Iterator.html) on, so we can just walk through every item in the array and manipulate them
    // println!("Array length: {}", this.unit_accessory_array.len());

    this.unit_accessory_array
        .iter_mut()
        .for_each(|item| {
            *item = UnitAccessory::instantiate()
                .map(|acc| {
                    acc.index = 0 as i32;
                    acc
                })
                .unwrap();
        });
}

#[unity::hook("App", "UnitAccessoryList", "Serialize")]
pub fn unitaccessorylist_serialize_hook(this: &mut UnitAccessoryList, stream: &mut Stream, _method_info: OptionalMethod) {
    stream.write_int(0).expect("Could not write version number when serializing UnitAccessoryList");

    // TODO: Simplify by calling serialize on the UnitAccessoryList directly
    this.unit_accessory_array[..4].iter_mut()
        .for_each(|curr_acc| {
            curr_acc.serialize(stream);
        });
}

#[unity::hook("App", "UnitAccessoryList", "Deserialize")]
pub fn unitaccessorylist_deserialize_hook(this: &mut UnitAccessoryList, stream: &mut Stream, _method_info: OptionalMethod) {
    this.unit_accessory_array
            .iter_mut()
            .for_each(|curr_acc| {
                curr_acc.index = 0;
            });

    let version_check = stream.read_int().expect("Could not read the version from the UnitAccessoryList block in the savefile");

    if version_check > 0 {
        // Deserializes as many items as there are in the array
        this.unit_accessory_array.iter_mut()
            .for_each(|curr_acc| {
                curr_acc.deserialize(stream);
            });
        // Unequips all accessories this first load because apparently accessories 
        // can get stuck equipped due to the Kinds being changed since the save was made.
        this.unit_accessory_array
            .iter_mut()
            .for_each(|curr_acc| {
                curr_acc.index = 0;
            });
    } else {
        // Just deserializes the 4 original items
        this.unit_accessory_array[..4].iter_mut()
            .for_each(|curr_acc| {
                curr_acc.deserialize(stream);
            });
        
        
    }
}

#[skyline::main(name = "SaveRevert")]
pub fn main() {
    // Install a panic handler for your plugin, allowing you to customize what to do if there's an issue in your code.
    std::panic::set_hook(Box::new(|info| {
        let location = info.location().unwrap();

        // Some magic thing to turn what was provided to the panic into a string. Don't mind it too much.
        // The message will be stored in the msg variable for you to use.
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<Any>",
                }
            },
        };

        // This creates a new String with a message of your choice, writing the location of the panic and its message inside of it.
        // Note the \0 at the end. This is needed because show_error is a C function and expects a C string.
        // This is actually just a result of bad old code and shouldn't be necessary most of the time.
        let err_msg = format!(
            "SaveRevert has panicked at '{}' with the following message:\n{}\0",
            location,
            msg
        );

        // We call the native Error dialog of the Nintendo Switch with this convenient method.
        // The error code is set to 69 because we do need a value, while the first message displays in the popup and the second shows up when pressing Details.
        skyline::error::show_error(
            69,
            "Custom plugin has panicked! Please open the details and send a screenshot to the developer, then close the game.\n\0",
            err_msg.as_str(),
        );
    }));

    skyline::install_hooks!(
        unitaccessorylist_serialize_hook,
        unitaccessorylist_deserialize_hook,
        unitaccessorylist_get_count,
        unitaccessorylist_ctor_hook
    );

    //Patches the length of UnitAccessoryList in it's ctor function.  This is necessary in order to properly load an edited save.
    skyline::patching::Patch::in_text(0x01f61c00).bytes(&[0x01, 0x02, 0x80, 0x52]).expect("Couldn’t patch that shit for some reasons");

}
