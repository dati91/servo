/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::PermissionStatusBinding::PermissionDescriptor;
use dom::bindings::codegen::Bindings::PermissionsBinding::{self, PermissionsMethods};
use dom::bindings::error::Error;
use dom::bindings::js::Root;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use dom::permissionstatus::{PermissionDescriptorType, PermissionStatus};
use dom::promise::Promise;
use js::conversions::{ConversionResult, FromJSValConvertible};
use js::jsapi::{JSContext, JSObject};
use js::jsval::{ObjectValue, UndefinedValue};
use std::rc::Rc;

const CONVERSION_ERROR: &'static str = "Can't convert to an IDL value of type PermissionDescriptor";

// https://w3c.github.io/permissions/#permissions
#[dom_struct]
pub struct Permissions {
    reflector_: Reflector,
}

impl Permissions {
    pub fn new_inherited() -> Permissions {
        Permissions {
            reflector_: Reflector::new(),
        }
    }

    pub fn new(global: &GlobalScope) -> Root<Permissions> {
        reflect_dom_object(box Permissions::new_inherited(),
                           global,
                           PermissionsBinding::Wrap)
    }
}

#[allow(unsafe_code)]
fn create_root_descriptor(cx: *mut JSContext,
                          permission_descriptor_obj: *mut JSObject)
                          -> Result<PermissionDescriptor, String> {
    rooted!(in(cx) let mut property = UndefinedValue());
    property.handle_mut().set(ObjectValue(permission_descriptor_obj));
    unsafe {
        match PermissionDescriptor::from_jsval(cx, property.handle(), ()) {
            Ok(ConversionResult::Success(descriptor)) => Ok(descriptor),
            Ok(ConversionResult::Failure(message)) => Err(String::from(message)),
            Err(_) => Err(String::from(CONVERSION_ERROR)),
        }
    }
}

// https://w3c.github.io/permissions/#dom-permissions-query
fn sync_default_permission_query_call(global: &GlobalScope,
                                      descriptor: PermissionDescriptor,
                                      promise: &Rc<Promise>,
                                      cx: *mut JSContext) {
    // Step 5.
    let status = PermissionStatus::create_from_descriptor(global, PermissionDescriptorType::Default(descriptor));

    // Step 6.
    // NOTE: `status.query` was initialized to descriptor in the previous statement.
    // We can use it here, instead cloning descriptor.
    status.permission_query(&*status.get_query().borrow());

    // Step 7.
    promise.resolve_native(cx, &status);
}

// https://w3c.github.io/permissions/#dom-permissions-request
fn sync_default_permission_request_call(global: &GlobalScope,
                                        descriptor: PermissionDescriptor,
                                        promise: &Rc<Promise>,
                                        cx: *mut JSContext) {
    // Step 5.
    let status = PermissionStatus::create_from_descriptor(global, PermissionDescriptorType::Default(descriptor));

    // Step 6.
    // NOTE: `status.query` was initialized to descriptor in the previous statement.
    // We can use it here, instead cloning descriptor.
    if let Err(err) = status.permission_request(&*status.get_query().borrow()) {
        // Step 7.
        return promise.reject_error(cx, err);
    }

    // Step 8.
    promise.resolve_native(cx, &status);
}

impl PermissionsMethods for Permissions {
    #[allow(unrooted_must_root)]
    #[allow(unsafe_code)]
    // https://w3c.github.io/permissions/#dom-permissions-query
    unsafe fn Query(&self, cx: *mut JSContext, permissionDesc: *mut JSObject) -> Rc<Promise> {
        // Step 3.
        let p = Promise::new(&self.global());

        // Step 1.
        let root_desc = match create_root_descriptor(cx, permissionDesc) {
            Ok(descriptor) => descriptor,
            Err(message) => {
                p.reject_error(cx, Error::Type(message));
                return p;
            },
        };

        // Step 2.
        // TODO: Add support for not default cases.
        match root_desc.name {
            _ => {
                // Step 4: TODO: Add an async implementation instead of sync_default_permission_query_call
                sync_default_permission_query_call(&self.global(), root_desc, &p, cx);
            },
        };

        return p;
    }

    #[allow(unrooted_must_root)]
    #[allow(unsafe_code)]
    // https://w3c.github.io/permissions/#dom-permissions-request
    unsafe fn Request(&self, cx: *mut JSContext, permissionDesc: *mut JSObject) -> Rc<Promise> {
        // Step 3.
        let p = Promise::new(&self.global());

        // Step 1.
        let root_desc = match create_root_descriptor(cx, permissionDesc) {
            Ok(descriptor) => descriptor,
            Err(message) => {
                p.reject_error(cx, Error::Type(message));
                return p;
            },
        };

        // Step 2.
        // TODO: Add support for not default cases.
        match root_desc.name {
            _ => {
                // Step 4: TODO: Add an async implementation instead of sync_default_permission_request_call
                sync_default_permission_request_call(&self.global(), root_desc, &p, cx);
            },
        };

        return p;
    }

    #[allow(unrooted_must_root)]
    #[allow(unsafe_code)]
    // https://w3c.github.io/permissions/#dom-permissions-revoke
    unsafe fn Revoke(&self, cx: *mut JSContext, permissionDesc: *mut JSObject) -> Rc<Promise> {
        let p = Promise::new(&self.global());

        // Step 1.
        let root_desc = match create_root_descriptor(cx, permissionDesc) {
            Ok(descriptor) => descriptor,
            Err(message) => {
                p.reject_error(cx, Error::Type(message));
                return p;
            },
        };

        // Step 2.
        // TODO: Add support for not default cases.
        match root_desc.name {
            _ => {
                // TODO: Clarify Step 3 - 4
                // NOTE(zakorgy): The steps are not clear about that we should call here a permission revoke method
                // but it is mentioned in `https://w3c.github.io/permissions/#permission-revocation-algorithm`:
                // "Run by Permissions' revoke() method"
            },
        };
        // Step 5.
        return self.Query(cx, permissionDesc);
    }
}
