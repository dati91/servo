/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::clone::Clone;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::EventHandlerBinding::EventHandlerNonNull;
use dom::bindings::codegen::Bindings::PermissionStatusBinding::{self, PermissionDescriptor, PermissionName};
use dom::bindings::codegen::Bindings::PermissionStatusBinding::{PermissionState, PermissionStatusMethods};
use dom::bindings::error::ErrorResult;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, reflect_dom_object};
use dom::bindings::trace::JSTraceable;
use dom::eventtarget::EventTarget;
use dom::globalscope::GlobalScope;
use heapsize::HeapSizeOf;
use js::jsapi::{JS_GetIsSecureContext, JSObject, JSTracer};
use js::rust::get_object_compartment;

// Enum for storing different type of PermissionDescriptors in the same type.
#[derive(JSTraceable, HeapSizeOf)]
pub enum PermissionDescriptorType {
    // TODO(zakorgy): Finish this list.
    Undefined,
    Default(PermissionDescriptor),
}

impl HeapSizeOf for PermissionDescriptor {
    fn heap_size_of_children(&self) -> usize {
        self.name.heap_size_of_children()
    }
}

#[allow(unsafe_code)]
unsafe impl JSTraceable for PermissionDescriptor {
    unsafe fn trace(&self, trc: *mut JSTracer) {
        self.name.trace(trc);
    }
}

// https://w3c.github.io/permissions/#permissionstatus
#[dom_struct]
pub struct PermissionStatus {
    eventtarget: EventTarget,
    state: DOMRefCell<PermissionState>,
    query: DOMRefCell<PermissionDescriptorType>,
}

impl PermissionStatus {
    pub fn new_inherited() -> PermissionStatus {
        PermissionStatus {
            eventtarget: EventTarget::new_inherited(),
            state: DOMRefCell::new(PermissionState::Denied),
            query: DOMRefCell::new(PermissionDescriptorType::Undefined),
        }
    }

    pub fn new(global: &GlobalScope) -> Root<PermissionStatus> {
        reflect_dom_object(box PermissionStatus::new_inherited(),
                           global,
                           PermissionStatusBinding::Wrap)
    }

    fn set_query(&self, permission_descriptor: PermissionDescriptorType) -> Root<PermissionStatus>  {
        *self.query.borrow_mut() = permission_descriptor;
        Root::from_ref(self)
    }

    pub fn get_query(&self) -> &DOMRefCell<PermissionDescriptorType> {
        &self.query
    }

    // https://w3c.github.io/permissions/#create-a-permissionstatus
    pub fn create_from_descriptor(global: &GlobalScope,
                                  permission_desc: PermissionDescriptorType)
                                  -> Root<PermissionStatus> {
        let permission_status = PermissionStatus::new(global);
        permission_status.set_query(permission_desc)
    }

    // https://w3c.github.io/permissions/#boolean-permission-query-algorithm
    pub fn permission_query(&self, permission_desc: &PermissionDescriptorType) {
        // Step 1.
        *self.state.borrow_mut() =
            get_descriptors_permission_state(permission_desc, self.eventtarget.reflector().get_jsobject().get());
    }

    // https://w3c.github.io/permissions/#boolean-permission-request-algorithm
    pub fn permission_request(&self, permission_desc: &PermissionDescriptorType) -> ErrorResult {
        // Step 1.
        self.permission_query(permission_desc);
        // Step 2.
        match *self.state.borrow() {
            PermissionState::Prompt => {
                // TODO: Step 3: Ask the users's permission.
                // https://w3c.github.io/permissions/#request-permission-to-use

                // Step 4.
                return Ok(self.permission_query(permission_desc));
            },
            _ => {
                return Ok(());
            },
        }
    }
}

#[allow(unsafe_code)]
// https://w3c.github.io/permissions/#permission-state
fn get_descriptors_permission_state(descriptor: &PermissionDescriptorType, obj: *mut JSObject) -> PermissionState {
    // TODO: Step 1: If settings wasn’t passed, set it to the current settings object.

    let name = match descriptor {
        // TODO(zakorgy): Finish this list.
        &PermissionDescriptorType::Default(ref desc) => desc.name,
        &PermissionDescriptorType::Undefined => return PermissionState::Denied,
    };

    unsafe {
        // Step 2.
        if !(JS_GetIsSecureContext(get_object_compartment(obj)) || allowed_in_nonsecure_contexts(name)) {
            return PermissionState::Denied;
        }
    }
    // TODO: Step 3: Store the invocation results
    // TODO: Step 4: Make interaction with the user to discover the user's intent.
    PermissionState::Granted
}


// https://w3c.github.io/permissions/#allowed-in-non-secure-contexts
fn allowed_in_nonsecure_contexts(permission_name: PermissionName) -> bool {
    match permission_name {
        // https://w3c.github.io/permissions/#dom-permissionname-geolocation
        PermissionName::Geolocation => true,
        // https://w3c.github.io/permissions/#dom-permissionname-notifications
        PermissionName::Notifications => true,
        // https://w3c.github.io/permissions/#dom-permissionname-push
        PermissionName::Push => false,
        // https://w3c.github.io/permissions/#dom-permissionname-midi
        PermissionName::Midi => true,
        // https://w3c.github.io/permissions/#dom-permissionname-camera
        PermissionName::Camera => false,
        // https://w3c.github.io/permissions/#dom-permissionname-microphone
        PermissionName::Microphone => false,
        // https://w3c.github.io/permissions/#dom-permissionname-speaker
        PermissionName::Speaker => false,
        // https://w3c.github.io/permissions/#dom-permissionname-device-info
        PermissionName::Device_info => false,
        // https://w3c.github.io/permissions/#dom-permissionname-background-sync
        PermissionName::Background_sync => false,
        // https://webbluetoothcg.github.io/web-bluetooth/#dom-permissionname-bluetooth
        PermissionName::Bluetooth => false,
        // https://storage.spec.whatwg.org/#dom-permissionname-persistent-storage
        PermissionName::Persistent_storage => false,
    }
}

impl PermissionStatusMethods for PermissionStatus {
    // https://w3c.github.io/permissions/#dom-permissionstatus-state
    fn State(&self) -> PermissionState {
        self.state.borrow().clone()
    }

    // https://w3c.github.io/permissions/#dom-permissionstatus-onchange
    event_handler!(onchange, GetOnchange, SetOnchange);
}
