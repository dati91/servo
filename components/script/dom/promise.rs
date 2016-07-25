/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::callback::CallbackContainer;
use dom::bindings::codegen::Bindings::PromiseBinding::AnyCallback;
use dom::bindings::error::Fallible;
use dom::bindings::global::GlobalRef;
//use dom::bindings::js::{JS, MutNullableHeap, Root};
use dom::bindings::reflector::{Reflectable, Reflector};
use js::jsapi::{JSAutoCompartment, CallArgs, HandleValueArray, JS_GetFunctionObject, JS_NewFunction, JS_ReportPendingException};
use js::jsapi::{JSContext, JSObject, HandleValue, HandleObject, IsPromiseObject, Call};
use js::jsapi::{CallOriginalPromiseResolve, CallOriginalPromiseReject, CallOriginalPromiseThen};
use js::jsapi::{MutableHandleObject, NewPromiseObject, ResolvePromise, RejectPromise, JS_ClearPendingException};
use js::jsval::{JSVal, UndefinedValue};
use std::ptr;
use std::rc::Rc;
use libc::c_void;

#[dom_struct]
pub struct Promise {
    reflector: Reflector,
}

impl Promise {
    #[allow(unsafe_code)]
    pub fn new(global: GlobalRef) -> Rc<Promise> {
        println!("new");
        let cx = global.get_cx();
        rooted!(in(cx) let mut obj = ptr::null_mut());
        unsafe {
            Promise::create_js_promise(cx, HandleObject::null(), obj.handle_mut());
        }
        Promise::new_with_js_promise(obj.handle())
    }

    #[allow(unsafe_code, unrooted_must_root)]
    fn new_with_js_promise(obj: HandleObject) -> Rc<Promise> {
        println!("new_with_js_promise");
        unsafe {
            assert!(IsPromiseObject(obj));
        }
        let mut promise = Promise {
            reflector: Reflector::new(),
        };
        promise.init_reflector(obj.get());
        Rc::new(promise)
    }

    #[allow(unsafe_code)]
    unsafe fn create_js_promise(cx: *mut JSContext, proto: HandleObject, mut obj: MutableHandleObject) {
        println!("create_js_promise");
        let do_nothing_func = JS_NewFunction(cx, Some(do_nothing_promise_executor), /* nargs = */ 2,
                                             /* flags = */ 0, ptr::null());
        assert!(!do_nothing_func.is_null());
        rooted!(in(cx) let do_nothing_obj = JS_GetFunctionObject(do_nothing_func));
        assert!(!do_nothing_obj.handle().is_null());
        *obj = NewPromiseObject(cx, do_nothing_obj.handle(), proto);
        assert!(!obj.is_null());
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn Resolve(global: GlobalRef,
                   cx: *mut JSContext,
                   value: HandleValue) -> Fallible<Rc<Promise>> {
        println!("Resolve");
        let _ac = JSAutoCompartment::new(cx, global.reflector().get_jsobject().get());
        rooted!(in(cx) let p = unsafe { CallOriginalPromiseResolve(cx, value) });
        assert!(!p.handle().is_null());
        Ok(Promise::new_with_js_promise(p.handle()))
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn Reject(global: GlobalRef,
                  cx: *mut JSContext,
                  value: HandleValue) -> Fallible<Rc<Promise>> {
        println!("Reject");
        let _ac = JSAutoCompartment::new(cx, global.reflector().get_jsobject().get());
        rooted!(in(cx) let p = unsafe { CallOriginalPromiseReject(cx, value) });
        assert!(!p.handle().is_null());
        Ok(Promise::new_with_js_promise(p.handle()))
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn MaybeResolve(&self,
                        cx: *mut JSContext,
                        value: HandleValue) {
        println!("MaybeResolve");
        unsafe {
            rooted!(in(cx) let p = self.promise_obj());
            if !ResolvePromise(cx, p.handle(), value) {
                JS_ClearPendingException(cx);
            }
        }
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn MaybeReject(&self,
                       cx: *mut JSContext,
                       value: HandleValue) {
        println!("MaybeReject");
        unsafe {
            rooted!(in(cx) let p = self.promise_obj());
            if !RejectPromise(cx, p.handle(), value) {
                JS_ClearPendingException(cx);
            }
        }
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn Then(&self,
                cx: *mut JSContext,
                _callee: HandleObject,
                cb_resolve: AnyCallback,
                cb_reject: AnyCallback,
                mut result: MutableHandleObject) {
        unsafe {
            println!("then promise");
            rooted!(in(cx) let promise = self.promise_obj());
            println!("then resolve");
            rooted!(in(cx) let resolve = cb_resolve.callback());
            println!("then reject");
            rooted!(in(cx) let reject = cb_reject.callback());
            println!("then res");
            rooted!(in(cx) let mut res =
                CallOriginalPromiseThen(cx, promise.handle(), resolve.handle(), reject.handle()));
            println!("then result");
            result = res.handle_mut();
        }
    }

    #[allow(unsafe_code)]
    fn promise_obj(&self) -> *mut JSObject {
        println!("promise_obj");
        let obj = self.reflector().get_jsobject();
        unsafe {
            if IsPromiseObject(obj) {
                // TODO we should call this somehow
                //ExposeObjectToActiveJS(obj.get());
            }
        }
        obj.get()
    }
}

// src/shell/js.cpp 629-657
pub fn enqueue_job(cx: *mut JSContext,
                   job: HandleObject,
                   _data: *const c_void) {
    // RootedObject job(cx);
    //rooted!(in(cx) let job = job);

    // UndefinedHandleValue
    rooted!(in(cx) let uval = UndefinedValue());

    // JS::HandleValueArray args(JS::HandleValueArray::empty());
    rooted!(in(cx) let args = HandleValueArray::empty());

    // RootedValue rval(cx);
    rooted!(in(cx) let rval = UndefinedValue());

    // AutoCompartment ac(cx, job);
    let _ac = JSAutoCompartment::new(cx, job.get());

    // if (!JS::Call(cx, UndefinedHandleValue, job, args, &rval))
    if !Call(cx, uval.handle(), job, args.handle(), rval.handle_mut()) {
        JS_ReportPendingException(cx);
    }
}

#[allow(unsafe_code)]
unsafe extern fn do_nothing_promise_executor(_cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
    println!("do_nothing_promise_executor");
    let args = CallArgs::from_vp(vp, argc);
    *args.rval() = UndefinedValue();
    true
}
