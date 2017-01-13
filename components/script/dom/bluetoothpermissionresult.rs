/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use bluetooth_traits::{BluetoothRequest, BluetoothResponse};
use bluetooth_traits::scanfilter::{BluetoothScanfilter, BluetoothScanfilterSequence};
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::BluetoothPermissionResultBinding::{self, BluetoothPermissionResultMethods};
use dom::bindings::codegen::Bindings::NavigatorBinding::NavigatorBinding::NavigatorMethods;
use dom::bindings::codegen::Bindings::PermissionStatusBinding::{PermissionState, PermissionStatusMethods};
use dom::bindings::codegen::Bindings::WindowBinding::WindowBinding::WindowMethods;
use dom::bindings::error::{Error, ErrorResult};
use dom::bindings::inheritance::Castable;
use dom::bindings::js::{JS, Root};
use dom::bindings::reflector::{DomObject, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::bluetooth::{AsyncBluetoothListener, Bluetooth};
use dom::bluetooth::{canonicalize_filter, get_allowed_devices, response_async, OPTIONS_ERROR};
use dom::bluetoothdevice::BluetoothDevice;
use dom::globalscope::GlobalScope;
use dom::permissionstatus::{PermissionStatus, PermissionDescriptorType};
use dom::promise::Promise;
use ipc_channel::ipc::{self, IpcSender};
use js::jsapi::JSContext;
use std::rc::Rc;

const DESCRIPTOR_TYPE_ERROR: &'static str = "Wrong type of descriptor in argument list.";

// https://webbluetoothcg.github.io/web-bluetooth/#bluetoothpermissionresult
#[dom_struct]
pub struct BluetoothPermissionResult {
    parent: PermissionStatus,
    devices: DOMRefCell<Vec<JS<BluetoothDevice>>>,
}

impl BluetoothPermissionResult {
    pub fn new_inherited() -> BluetoothPermissionResult {
        BluetoothPermissionResult {
            parent: PermissionStatus::new_inherited(),
            devices: DOMRefCell::new(Vec::new()),
        }
    }

    pub fn new(global: &GlobalScope) -> Root<BluetoothPermissionResult> {
        reflect_dom_object(box BluetoothPermissionResult::new_inherited(),
                           global,
                           BluetoothPermissionResultBinding::Wrap)
    }

    fn get_bluetooth(&self) -> Root<Bluetooth> {
        self.global().as_window().Navigator().Bluetooth()
    }

    fn get_bluetooth_thread(&self) -> IpcSender<BluetoothRequest> {
        self.global().as_window().bluetooth_thread()
    }

    // https://w3c.github.io/permissions/#create-a-permissionstatus
    pub fn create_from_descriptor(global: &GlobalScope,
                                  descriptor: PermissionDescriptorType)
                                  -> Root<BluetoothPermissionResult> {
        let bt_permission_result = BluetoothPermissionResult::new(global);
        bt_permission_result.upcast::<PermissionStatus>().set_query(descriptor);
        bt_permission_result
    }

    #[allow(unrooted_must_root)]
    // https://webbluetoothcg.github.io/web-bluetooth/#query-the-bluetooth-permission
    pub fn permission_query(&self, promise: &Rc<Promise>, cx: *mut JSContext) {
        let descriptor = self.parent.get_query().borrow();
        let bluetoth_descriptor = match *descriptor {
            PermissionDescriptorType::Bluetooth(ref d) => d,
            _ => return promise.reject_error(cx, (Error::Type(DESCRIPTOR_TYPE_ERROR.to_owned()))),
        };

        // Step 2.
        self.parent.permission_query(&*descriptor);

        // Step 3.
        if let PermissionState::Denied = self.parent.State() {
            *self.devices.borrow_mut() = Vec::new();
            return promise.resolve_native(cx, &self.parent);
        }

        // Step 4.
        let mut matching_devices: Vec<JS<BluetoothDevice>> = Vec::new();

        // TODO: Step 5: Create a map between the current setting object and BluetoothPermissionData
        // extra permission data, which replaces the exisitng EXTRA_PERMISSION_DATA global variable.
        // For this also use the extra permission data constraints from the specification:
        // https://webbluetoothcg.github.io/web-bluetooth/#dictdef-bluetoothpermissiondata

        let bluetooth = self.get_bluetooth();
        let device_map = bluetooth.get_device_map().borrow();

        // Step 5.
        let allowed_devices = get_allowed_devices();

        // Step 6.
        for allowed_device in allowed_devices {
            // Step 6.1.
            if let Some(ref id) = bluetoth_descriptor.deviceId {
                if &allowed_device.deviceId != id {
                    continue;
                }
            }
            let device_id = String::from(allowed_device.deviceId.as_ref());
            if let Some(ref filters) = bluetoth_descriptor.filters {
                let mut scan_filters: Vec<BluetoothScanfilter> = Vec::new();

                // NOTE(zakorgy): This canonicalizing step is missing from the specification.
                // But there is an issue for this: https://github.com/WebBluetoothCG/web-bluetooth/issues/347
                for filter in filters {
                    match canonicalize_filter(&filter) {
                        Ok(f) => scan_filters.push(f),
                        Err(err) => return promise.reject_error(cx, err),
                    }
                }

                // Step 6.2.
                // Instead of creating an internal slot we send an ipc message to the Bluetooth thread
                // to check if one of the filters matches.
                let (sender, receiver) = ipc::channel().unwrap();
                self.get_bluetooth_thread()
                    .send(BluetoothRequest::MatchesFilter(device_id.clone(),
                                                          BluetoothScanfilterSequence::new(scan_filters),
                                                          sender)).unwrap();

                match receiver.recv().unwrap() {
                    Ok(true) => (),
                    Ok(false) => continue,
                    Err(err) => return promise.reject_error(cx, Error::from(err)),
                };
            }

            // Step 6.4.
            // TODO: Implement this correctly, not just using device ids here.
            // https://webbluetoothcg.github.io/web-bluetooth/#get-the-bluetoothdevice-representing
            if let Some(ref device) = device_map.get(&device_id) {
                matching_devices.push(JS::from_ref(&**device.clone()));
            }
        }

        // Step 7.
        *self.devices.borrow_mut() = matching_devices;
    }

    // https://webbluetoothcg.github.io/web-bluetooth/#bluetoothpermissionresult
    // NOTE(zakorgy): There is no link for this algorithm until this PR for the spec is pending:
    // https://github.com/WebBluetoothCG/web-bluetooth/pull/349
    pub fn permission_request(&self, promise: &Rc<Promise>) -> ErrorResult {
        let descriptor = self.parent.get_query().borrow();
        let options = match *descriptor {
            PermissionDescriptorType::Bluetooth(ref d) => d,
            _ => return Err(Error::Type(DESCRIPTOR_TYPE_ERROR.to_owned())),
        };

        // Step 1.
        if options.filters.is_some() == options.acceptAllDevices {
            return Err(Error::Type(OPTIONS_ERROR.to_owned()));
        }

        // Step 2.
        let sender = response_async(promise, self);
        let bluetooth = self.get_bluetooth();
        Ok(bluetooth.request_bluetooth_devices(promise, &options.filters, &options.optionalServices, sender))

        // NOTE: Step 3 is in `handle_response` function.
    }
}


// TODO: Make a BluetoothResponse variant which returns with a vector of devices that matches.
impl AsyncBluetoothListener for BluetoothPermissionResult {
    fn handle_response(&self, response: BluetoothResponse, promise_cx: *mut JSContext, promise: &Rc<Promise>) {
        match response {
            BluetoothResponse::RequestDevice(device) => {
                let bluetooth = &self.get_bluetooth();
                let mut device_instance_map = bluetooth.get_device_map().borrow_mut();
                if let Some(existing_device) = device_instance_map.get(&device.id) {
                    // https://webbluetoothcg.github.io/web-bluetooth/#bluetoothpermissionresult
                    // Step 3.
                    *self.devices.borrow_mut() = vec!(JS::from_ref(&**existing_device));

                    // https://w3c.github.io/permissions/#dom-permissions-request
                    // Step 8.
                    return promise.resolve_native(promise_cx, &self.parent);
                }
                let bt_device = BluetoothDevice::new(&self.global(),
                                                     DOMString::from(device.id.clone()),
                                                     device.name.map(DOMString::from),
                                                     bluetooth);
                device_instance_map.insert(device.id, JS::from_ref(&bt_device));
                // https://webbluetoothcg.github.io/web-bluetooth/#bluetoothpermissionresult
                // Step 3.
                *self.devices.borrow_mut() = vec!(JS::from_ref(&bt_device));

                // https://w3c.github.io/permissions/#dom-permissions-request
                // Step 8.
                promise.resolve_native(promise_cx, &self.parent);
            },
            _ => promise.reject_error(promise_cx, Error::Type("Something went wrong...".to_owned())),
        }
    }
}

impl BluetoothPermissionResultMethods for BluetoothPermissionResult {
    // https://webbluetoothcg.github.io/web-bluetooth/#dom-bluetoothpermissionresult-devices
    fn Devices(&self) -> Vec<Root<BluetoothDevice>> {
        let device_vec: Vec<Root<BluetoothDevice>> =
            self.devices.borrow().iter().map(|d| Root::from_ref(&**d)).collect();
        device_vec
    }
}
