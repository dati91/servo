/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::PromiseBinding::AnyCallback;
use dom::bindings::error::Fallible;
use dom::bindings::global::GlobalRef;
use dom::bindings::reflector::{Reflectable, Reflector};
use js::jsapi::{JSAutoCompartment, CallArgs, JS_GetFunctionObject, JS_NewFunction};
use js::jsapi::{JSContext, HandleValue, HandleObject, IsPromiseObject, CallOriginalPromiseResolve};
use js::jsapi::{MutableHandleObject, NewPromiseObject, JS_WrapObject};
use js::jsval::{JSVal, UndefinedValue};
use std::ptr;
use std::rc::Rc;

#[dom_struct]
pub struct Promise {
    reflector: Reflector
}

impl Promise {
    #[allow(unsafe_code)]
    pub fn new(global: GlobalRef) -> Rc<Promise> {
        let cx = global.get_cx();
        rooted!(in(cx) let mut obj = ptr::null_mut());
        unsafe {
            Promise::create_js_promise(cx, HandleObject::null(), obj.handle_mut());
        }
        Promise::new_with_js_promise(obj.handle())
    }

    #[allow(unsafe_code, unrooted_must_root)]
    fn new_with_js_promise(obj: HandleObject) -> Rc<Promise> {
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
        let do_nothing_func = JS_NewFunction(cx, Some(do_nothing_promise_executor), 2, 0, ptr::null());
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
        let _ac = JSAutoCompartment::new(cx, global.reflector().get_jsobject().get());
        rooted!(in(cx) let p = unsafe { CallOriginalPromiseResolve(cx, value) });
        assert!(!p.handle().is_null());
        Ok(Promise::new_with_js_promise(p.handle()))
    }

    #[allow(unrooted_must_root, unsafe_code)]
    pub fn Then(cx: *mut JSContext,
                callee: HandleObject,
                cb_resolve: AnyCallback,
                cb_reject: AnyCallback,
                mut result: MutableHandleObject) {

        // todo
        /*rooted!(in(cx) let promise = ptr::null());
        if !JS_WrapObject(cx, ???) {

        }*/


        // firefox

        /*JS::Rooted<JSObject*> promise(aCx, PromiseObj());
        if (!JS_WrapObject(aCx, &promise)) {
        }

        JS::Rooted<JSObject*> resolveCallback(aCx);
        if (aResolveCallback) {
        resolveCallback = aResolveCallback->Callback();
        if (!JS_WrapObject(aCx, &resolveCallback)) {
          aRv.NoteJSContextException(aCx);
          return;
        }
        }

        JS::Rooted<JSObject*> rejectCallback(aCx);
        if (aRejectCallback) {
        rejectCallback = aRejectCallback->Callback();
        if (!JS_WrapObject(aCx, &rejectCallback)) {
          aRv.NoteJSContextException(aCx);
          return;
        }
        }

        JS::Rooted<JSObject*> retval(aCx);
        retval = JS::CallOriginalPromiseThen(aCx, promise, resolveCallback,
                                           rejectCallback);
        if (!retval) {
        aRv.NoteJSContextException(aCx);
        return;
        }

        aRetval.setObject(*retval);*/
    }

}

#[allow(unsafe_code)]
unsafe extern fn do_nothing_promise_executor(_cx: *mut JSContext, argc: u32, vp: *mut JSVal) -> bool {
    let args = CallArgs::from_vp(vp, argc);
    *args.rval() = UndefinedValue();
    true
}
