windows_core::imp::define_interface!(
    IClosable,
    IClosable_Vtbl,
    0x30d5a829_7fa4_4026_83bb_d75bae4ea99e
);
impl windows_core::RuntimeType for IClosable {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IClosable,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IClosable {
    pub fn Close(&self) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).Close)(windows_core::Interface::as_raw(this))
                .ok()
        }
    }
}
impl windows_core::RuntimeName for IClosable {
    const NAME: &'static str = "Windows.Foundation.IClosable";
}
pub trait IClosable_Impl: windows_core::IUnknownImpl {
    fn Close(&self) -> windows_core::Result<()>;
}
impl IClosable_Vtbl {
    pub const fn new<Identity: IClosable_Impl, const OFFSET: isize>() -> Self {
        unsafe extern "system" fn Close<Identity: IClosable_Impl, const OFFSET: isize>(
            this: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IClosable_Impl::Close(this).into()
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<Identity, IClosable, OFFSET>(),
            Close: Close::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IClosable as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IClosable_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Close: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnection,
    IMidiEndpointConnection_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000050000
);
impl windows_core::RuntimeType for IMidiEndpointConnection {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointConnection_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub LogMessageDataValidationErrorDetails:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetLogMessageDataValidationErrorDetails:
        unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    pub Open: unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub MessageProcessingPlugins: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub AddMessageProcessingPlugin: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub RemoveMessageProcessingPlugin: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub SendSingleMessagePacket: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u8,
        &MidiMessageStruct,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageWordArray: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u8,
        u32,
        *const u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageWords: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageWords2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageWords3: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendSingleMessageWords4: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    SendSingleMessageBuffer: usize,
    pub SendMultipleMessagesWordList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        *mut core::ffi::c_void,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendMultipleMessagesWordArray: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        *const u32,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendMultipleMessagesPacketList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendMultipleMessagesStructList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        *mut core::ffi::c_void,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    pub SendMultipleMessagesStructArray: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        *const MidiMessageStruct,
        *mut MidiSendMessageResults,
    ) -> windows_core::HRESULT,
    SendMultipleMessagesBuffer: usize,
    pub GetSupportedMaxMidiWordsPerTransmission:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnectionBasicSettings,
    IMidiEndpointConnectionBasicSettings_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000060000
);
impl windows_core::RuntimeType for IMidiEndpointConnectionBasicSettings {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointConnectionBasicSettings_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnectionBasicSettingsFactory,
    IMidiEndpointConnectionBasicSettingsFactory_Vtbl,
    0x0c61f471_153d_5a85_b10f_5815172b8b61
);
impl windows_core::RuntimeType for IMidiEndpointConnectionBasicSettingsFactory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointConnectionBasicSettingsFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        bool,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        bool,
        bool,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnectionSettings,
    IMidiEndpointConnectionSettings_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000020
);
impl windows_core::RuntimeType for IMidiEndpointConnectionSettings {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiEndpointConnectionSettings,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiEndpointConnectionSettings {
    pub fn WaitForEndpointReceiptOnSend(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).WaitForEndpointReceiptOnSend)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AutoReconnect(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AutoReconnect)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SettingsJson(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SettingsJson)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
}
impl windows_core::RuntimeName for IMidiEndpointConnectionSettings {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.IMidiEndpointConnectionSettings";
}
pub trait IMidiEndpointConnectionSettings_Impl: windows_core::IUnknownImpl {
    fn WaitForEndpointReceiptOnSend(&self) -> windows_core::Result<bool>;
    fn AutoReconnect(&self) -> windows_core::Result<bool>;
    fn SettingsJson(&self) -> windows_core::Result<windows_core::HSTRING>;
}
impl IMidiEndpointConnectionSettings_Vtbl {
    pub const fn new<Identity: IMidiEndpointConnectionSettings_Impl, const OFFSET: isize>() -> Self
    {
        unsafe extern "system" fn WaitForEndpointReceiptOnSend<
            Identity: IMidiEndpointConnectionSettings_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSettings_Impl::WaitForEndpointReceiptOnSend(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn AutoReconnect<
            Identity: IMidiEndpointConnectionSettings_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSettings_Impl::AutoReconnect(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SettingsJson<
            Identity: IMidiEndpointConnectionSettings_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSettings_Impl::SettingsJson(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiEndpointConnectionSettings,
                OFFSET,
            >(),
            WaitForEndpointReceiptOnSend: WaitForEndpointReceiptOnSend::<Identity, OFFSET>,
            AutoReconnect: AutoReconnect::<Identity, OFFSET>,
            SettingsJson: SettingsJson::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiEndpointConnectionSettings as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiEndpointConnectionSettings_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub WaitForEndpointReceiptOnSend:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub AutoReconnect:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SettingsJson: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnectionSource,
    IMidiEndpointConnectionSource_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000030
);
impl windows_core::RuntimeType for IMidiEndpointConnectionSource {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiEndpointConnectionSource,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiEndpointConnectionSource {
    pub fn RemoveEndpointDeviceDisconnected(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveEndpointDeviceDisconnected)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveEndpointDeviceReconnected(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveEndpointDeviceReconnected)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetTag)(
                windows_core::Interface::as_raw(this),
                value.param().abi(),
            )
            .ok()
        }
    }
    pub fn ConnectionId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ConnectionId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ConnectedEndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ConnectedEndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn Settings(&self) -> windows_core::Result<IMidiEndpointConnectionSettings> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Settings)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn IsOpen(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsOpen)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeName for IMidiEndpointConnectionSource {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.IMidiEndpointConnectionSource";
}
pub trait IMidiEndpointConnectionSource_Impl: windows_core::IUnknownImpl {
    fn RemoveEndpointDeviceDisconnected(&self, token: i64) -> windows_core::Result<()>;
    fn RemoveEndpointDeviceReconnected(&self, token: i64) -> windows_core::Result<()>;
    fn Tag(&self) -> windows_core::Result<windows_core::IInspectable>;
    fn SetTag(
        &self,
        value: windows_core::Ref<'_, windows_core::IInspectable>,
    ) -> windows_core::Result<()>;
    fn ConnectionId(&self) -> windows_core::Result<windows_core::GUID>;
    fn ConnectedEndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING>;
    fn Settings(&self) -> windows_core::Result<IMidiEndpointConnectionSettings>;
    fn IsOpen(&self) -> windows_core::Result<bool>;
}
impl IMidiEndpointConnectionSource_Vtbl {
    pub const fn new<Identity: IMidiEndpointConnectionSource_Impl, const OFFSET: isize>() -> Self {
        unsafe extern "system" fn RemoveEndpointDeviceDisconnected<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            token: i64,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointConnectionSource_Impl::RemoveEndpointDeviceDisconnected(this, token)
                    .into()
            }
        }
        unsafe extern "system" fn RemoveEndpointDeviceReconnected<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            token: i64,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointConnectionSource_Impl::RemoveEndpointDeviceReconnected(this, token)
                    .into()
            }
        }
        unsafe extern "system" fn Tag<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSource_Impl::Tag(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SetTag<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            value: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointConnectionSource_Impl::SetTag(this, core::mem::transmute_copy(&value))
                    .into()
            }
        }
        unsafe extern "system" fn ConnectionId<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut windows_core::GUID,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSource_Impl::ConnectionId(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn ConnectedEndpointDeviceId<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSource_Impl::ConnectedEndpointDeviceId(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn Settings<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSource_Impl::Settings(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn IsOpen<
            Identity: IMidiEndpointConnectionSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointConnectionSource_Impl::IsOpen(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiEndpointConnectionSource,
                OFFSET,
            >(),
            EndpointDeviceDisconnected: 0,
            RemoveEndpointDeviceDisconnected: RemoveEndpointDeviceDisconnected::<Identity, OFFSET>,
            EndpointDeviceReconnected: 0,
            RemoveEndpointDeviceReconnected: RemoveEndpointDeviceReconnected::<Identity, OFFSET>,
            Tag: Tag::<Identity, OFFSET>,
            SetTag: SetTag::<Identity, OFFSET>,
            ConnectionId: ConnectionId::<Identity, OFFSET>,
            ConnectedEndpointDeviceId: ConnectedEndpointDeviceId::<Identity, OFFSET>,
            Settings: Settings::<Identity, OFFSET>,
            IsOpen: IsOpen::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiEndpointConnectionSource as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiEndpointConnectionSource_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    EndpointDeviceDisconnected: usize,
    pub RemoveEndpointDeviceDisconnected:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    EndpointDeviceReconnected: usize,
    pub RemoveEndpointDeviceReconnected:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub Tag: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetTag: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub ConnectionId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub ConnectedEndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Settings: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub IsOpen:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointConnectionStatics,
    IMidiEndpointConnectionStatics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000050000
);
impl windows_core::RuntimeType for IMidiEndpointConnectionStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointConnectionStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetDeviceSelector: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SendMessageSucceeded: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        MidiSendMessageResults,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub SendMessageFailed: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        MidiSendMessageResults,
        *mut bool,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformation,
    IMidiEndpointDeviceInformation_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000a000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformation {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformation_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub EndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Name: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub ContainerId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub DeviceInstanceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    EndpointPurpose: usize,
    pub GetDeclaredEndpointInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
    ) -> windows_core::HRESULT,
    DeclaredEndpointInfoLastUpdateTime: usize,
    pub GetDeclaredDeviceIdentity: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut MidiDeclaredDeviceIdentity,
    ) -> windows_core::HRESULT,
    DeclaredDeviceIdentityLastUpdateTime: usize,
    GetDeclaredStreamConfiguration: usize,
    DeclaredStreamConfigurationLastUpdateTime: usize,
    GetDeclaredFunctionBlocks: usize,
    DeclaredFunctionBlocksLastUpdateTime: usize,
    GetGroupTerminalBlocks: usize,
    pub GetUserSuppliedInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::mem::MaybeUninit<MidiEndpointUserSuppliedInfo>,
    ) -> windows_core::HRESULT,
    GetTransportSuppliedInfo: usize,
    GetParentDeviceInformation: usize,
    GetContainerDeviceInformation: usize,
    pub Properties: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    GetNameTable: usize,
    FindAllAssociatedMidi1PortsForThisEndpoint: usize,
    FindAllAssociatedMidi1PortsForThisEndpoint2: usize,
    FindAssociatedMidi1PortForGroupForThisEndpoint: usize,
    FindAssociatedMidi1PortForGroupForThisEndpoint2: usize,
    Midi1PortNamingApproach: usize,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformation2,
    IMidiEndpointDeviceInformation2_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000a002
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformation2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiEndpointDeviceInformation2,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiEndpointDeviceInformation2 {
    pub fn IsMuted(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsMuted)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeName for IMidiEndpointDeviceInformation2 {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.IMidiEndpointDeviceInformation2";
}
pub trait IMidiEndpointDeviceInformation2_Impl: windows_core::IUnknownImpl {
    fn IsMuted(&self) -> windows_core::Result<bool>;
}
impl IMidiEndpointDeviceInformation2_Vtbl {
    pub const fn new<Identity: IMidiEndpointDeviceInformation2_Impl, const OFFSET: isize>() -> Self
    {
        unsafe extern "system" fn IsMuted<
            Identity: IMidiEndpointDeviceInformation2_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointDeviceInformation2_Impl::IsMuted(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiEndpointDeviceInformation2,
                OFFSET,
            >(),
            IsMuted: IsMuted::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiEndpointDeviceInformation2 as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformation2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub IsMuted:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformationAddedEventArgs,
    IMidiEndpointDeviceInformationAddedEventArgs_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000b000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformationAddedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformationAddedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub AddedDevice: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformationRemovedEventArgs,
    IMidiEndpointDeviceInformationRemovedEventArgs_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000c000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformationRemovedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformationRemovedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub EndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    DeviceInformationUpdate: usize,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformationStatics,
    IMidiEndpointDeviceInformationStatics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee000000a000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformationStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformationStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromEndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub FindAll: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    FindAll2: usize,
    FindAll3: usize,
    pub CreateFromAssociatedMidi1PortDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    )
        -> windows_core::HRESULT,
    CreateFromAssociatedMidi1PortNumber: usize,
    FindEndpointDeviceIdForAssociatedMidi1PortNumber: usize,
    FindAllForAssociatedMidi1PortName: usize,
    FindAllEndpointDeviceIdsForAssociatedMidi1PortName: usize,
    pub EndpointInterfaceClass: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub GetAdditionalPropertiesList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    DeviceMatchesFilter: usize,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformationUpdatedEventArgs,
    IMidiEndpointDeviceInformationUpdatedEventArgs_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000d000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformationUpdatedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformationUpdatedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub EndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub IsNameUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub IsEndpointInformationUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub IsDeviceIdentityUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub IsStreamConfigurationUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub AreFunctionBlocksUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub IsUserMetadataUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub AreAdditionalCapabilitiesUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub AreUniqueIdsUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    DeviceInformationUpdate: usize,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceInformationUpdatedEventArgs2,
    IMidiEndpointDeviceInformationUpdatedEventArgs2_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000d002
);
impl windows_core::RuntimeType for IMidiEndpointDeviceInformationUpdatedEventArgs2 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiEndpointDeviceInformationUpdatedEventArgs2,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiEndpointDeviceInformationUpdatedEventArgs2 {
    pub fn AreGroupTerminalBlocksUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AreGroupTerminalBlocksUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeName for IMidiEndpointDeviceInformationUpdatedEventArgs2 {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.IMidiEndpointDeviceInformationUpdatedEventArgs2";
}
pub trait IMidiEndpointDeviceInformationUpdatedEventArgs2_Impl: windows_core::IUnknownImpl {
    fn AreGroupTerminalBlocksUpdated(&self) -> windows_core::Result<bool>;
}
impl IMidiEndpointDeviceInformationUpdatedEventArgs2_Vtbl {
    pub const fn new<
        Identity: IMidiEndpointDeviceInformationUpdatedEventArgs2_Impl,
        const OFFSET: isize,
    >() -> Self {
        unsafe extern "system" fn AreGroupTerminalBlocksUpdated<
            Identity: IMidiEndpointDeviceInformationUpdatedEventArgs2_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointDeviceInformationUpdatedEventArgs2_Impl:: AreGroupTerminalBlocksUpdated ( this , ) { Ok ( ok__ ) => { result__ . write ( core::mem::transmute_copy ( & ok__ ) ) ;  windows_core::HRESULT ( 0 ) } Err ( err ) => err . into ( ) }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiEndpointDeviceInformationUpdatedEventArgs2,
                OFFSET,
            >(),
            AreGroupTerminalBlocksUpdated: AreGroupTerminalBlocksUpdated::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiEndpointDeviceInformationUpdatedEventArgs2 as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiEndpointDeviceInformationUpdatedEventArgs2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub AreGroupTerminalBlocksUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceWatcher,
    IMidiEndpointDeviceWatcher_Vtbl,
    0x8087b303_0519_31d1_c0de_dd000000e000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceWatcher {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceWatcher_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Start: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub Stop: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub EnumeratedEndpointDevices: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    Status: usize,
    Added: usize,
    pub RemoveAdded:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    Removed: usize,
    pub RemoveRemoved:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    Updated: usize,
    pub RemoveUpdated:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    EnumerationCompleted: usize,
    pub RemoveEnumerationCompleted:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    Stopped: usize,
    pub RemoveStopped:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiEndpointDeviceWatcherStatics,
    IMidiEndpointDeviceWatcherStatics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee000000e000
);
impl windows_core::RuntimeType for IMidiEndpointDeviceWatcherStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiEndpointDeviceWatcherStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Create: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    Create2: usize,
}
windows_core::imp::define_interface!(
    IMidiEndpointMessageProcessingPlugin,
    IMidiEndpointMessageProcessingPlugin_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000040
);
impl windows_core::RuntimeType for IMidiEndpointMessageProcessingPlugin {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiEndpointMessageProcessingPlugin,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiEndpointMessageProcessingPlugin {
    pub fn PluginId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PluginName(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginName)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetPluginName(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetPluginName)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn PluginTag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginTag)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn SetPluginTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetPluginTag)(
                windows_core::Interface::as_raw(this),
                value.param().abi(),
            )
            .ok()
        }
    }
    pub fn IsEnabled(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsEnabled)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetIsEnabled)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Initialize<P0>(&self, endpointconnection: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<IMidiEndpointConnectionSource>,
    {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).Initialize)(
                windows_core::Interface::as_raw(this),
                endpointconnection.param().abi(),
            )
            .ok()
        }
    }
    pub fn OnEndpointConnectionOpened(&self) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).OnEndpointConnectionOpened)(
                windows_core::Interface::as_raw(this),
            )
            .ok()
        }
    }
    pub fn ProcessIncomingMessage<P0>(
        &self,
        args: P0,
        skipfurtherlisteners: &mut bool,
        skipmainmessagereceivedevent: &mut bool,
    ) -> windows_core::Result<()>
    where
        P0: windows_core::Param<MidiMessageReceivedEventArgs>,
    {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).ProcessIncomingMessage)(
                windows_core::Interface::as_raw(this),
                args.param().abi(),
                skipfurtherlisteners,
                skipmainmessagereceivedevent,
            )
            .ok()
        }
    }
    pub fn Cleanup(&self) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).Cleanup)(windows_core::Interface::as_raw(this))
                .ok()
        }
    }
}
impl windows_core::RuntimeName for IMidiEndpointMessageProcessingPlugin {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.IMidiEndpointMessageProcessingPlugin";
}
pub trait IMidiEndpointMessageProcessingPlugin_Impl: windows_core::IUnknownImpl {
    fn PluginId(&self) -> windows_core::Result<windows_core::GUID>;
    fn PluginName(&self) -> windows_core::Result<windows_core::HSTRING>;
    fn SetPluginName(&self, value: &windows_core::HSTRING) -> windows_core::Result<()>;
    fn PluginTag(&self) -> windows_core::Result<windows_core::IInspectable>;
    fn SetPluginTag(
        &self,
        value: windows_core::Ref<'_, windows_core::IInspectable>,
    ) -> windows_core::Result<()>;
    fn IsEnabled(&self) -> windows_core::Result<bool>;
    fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()>;
    fn Initialize(
        &self,
        endpointConnection: windows_core::Ref<'_, IMidiEndpointConnectionSource>,
    ) -> windows_core::Result<()>;
    fn OnEndpointConnectionOpened(&self) -> windows_core::Result<()>;
    fn ProcessIncomingMessage(
        &self,
        args: windows_core::Ref<'_, MidiMessageReceivedEventArgs>,
        skipFurtherListeners: &mut bool,
        skipMainMessageReceivedEvent: &mut bool,
    ) -> windows_core::Result<()>;
    fn Cleanup(&self) -> windows_core::Result<()>;
}
impl IMidiEndpointMessageProcessingPlugin_Vtbl {
    pub const fn new<Identity: IMidiEndpointMessageProcessingPlugin_Impl, const OFFSET: isize>()
    -> Self {
        unsafe extern "system" fn PluginId<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut windows_core::GUID,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointMessageProcessingPlugin_Impl::PluginId(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn PluginName<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointMessageProcessingPlugin_Impl::PluginName(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SetPluginName<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            value: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::SetPluginName(
                    this,
                    core::mem::transmute::<&*mut libc::c_void, &windows_core::HSTRING>(&value),
                )
                .into()
            }
        }
        unsafe extern "system" fn PluginTag<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointMessageProcessingPlugin_Impl::PluginTag(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SetPluginTag<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            value: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::SetPluginTag(
                    this,
                    core::mem::transmute_copy(&value),
                )
                .into()
            }
        }
        unsafe extern "system" fn IsEnabled<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiEndpointMessageProcessingPlugin_Impl::IsEnabled(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SetIsEnabled<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            value: bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::SetIsEnabled(this, value).into()
            }
        }
        unsafe extern "system" fn Initialize<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            endpointconnection: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::Initialize(
                    this,
                    core::mem::transmute_copy(&endpointconnection),
                )
                .into()
            }
        }
        unsafe extern "system" fn OnEndpointConnectionOpened<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::OnEndpointConnectionOpened(this).into()
            }
        }
        unsafe extern "system" fn ProcessIncomingMessage<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            args: *mut core::ffi::c_void,
            skipfurtherlisteners: *mut bool,
            skipmainmessagereceivedevent: *mut bool,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::ProcessIncomingMessage(
                    this,
                    core::mem::transmute_copy(&args),
                    core::mem::transmute_copy(&skipfurtherlisteners),
                    core::mem::transmute_copy(&skipmainmessagereceivedevent),
                )
                .into()
            }
        }
        unsafe extern "system" fn Cleanup<
            Identity: IMidiEndpointMessageProcessingPlugin_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiEndpointMessageProcessingPlugin_Impl::Cleanup(this).into()
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiEndpointMessageProcessingPlugin,
                OFFSET,
            >(),
            PluginId: PluginId::<Identity, OFFSET>,
            PluginName: PluginName::<Identity, OFFSET>,
            SetPluginName: SetPluginName::<Identity, OFFSET>,
            PluginTag: PluginTag::<Identity, OFFSET>,
            SetPluginTag: SetPluginTag::<Identity, OFFSET>,
            IsEnabled: IsEnabled::<Identity, OFFSET>,
            SetIsEnabled: SetIsEnabled::<Identity, OFFSET>,
            Initialize: Initialize::<Identity, OFFSET>,
            OnEndpointConnectionOpened: OnEndpointConnectionOpened::<Identity, OFFSET>,
            ProcessIncomingMessage: ProcessIncomingMessage::<Identity, OFFSET>,
            Cleanup: Cleanup::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiEndpointMessageProcessingPlugin as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiEndpointMessageProcessingPlugin_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub PluginId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub PluginName: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetPluginName: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub PluginTag: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetPluginTag: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub IsEnabled:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetIsEnabled:
        unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    pub Initialize: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub OnEndpointConnectionOpened:
        unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub ProcessIncomingMessage: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub Cleanup: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage128,
    IMidiMessage128_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000040128
);
impl windows_core::RuntimeType for IMidiMessage128 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage128_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Word0: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord0: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word1: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord1: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word2: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord2: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word3: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord3: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage128Factory,
    IMidiMessage128Factory_Vtbl,
    0x6fb629df_e277_5ee8_b7b8_13a3528ba255
);
impl windows_core::RuntimeType for IMidiMessage128Factory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage128Factory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance3: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        *const u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage128Statics,
    IMidiMessage128Statics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000040128
);
impl windows_core::RuntimeType for IMidiMessage128Statics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage128Statics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        MidiMessageStruct,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage32,
    IMidiMessage32_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000040032
);
impl windows_core::RuntimeType for IMidiMessage32 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage32_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Word0: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord0: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage32Factory,
    IMidiMessage32Factory_Vtbl,
    0x47d7fd0f_7945_5283_b11b_a4f5cfbbf6f2
);
impl windows_core::RuntimeType for IMidiMessage32Factory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage32Factory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage32Statics,
    IMidiMessage32Statics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000040032
);
impl windows_core::RuntimeType for IMidiMessage32Statics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage32Statics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        MidiMessageStruct,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage64,
    IMidiMessage64_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000040064
);
impl windows_core::RuntimeType for IMidiMessage64 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage64_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Word0: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord0: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word1: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord1: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage64Factory,
    IMidiMessage64Factory_Vtbl,
    0x10b85e46_c6b8_593a_9385_11ae2fceb9f1
);
impl windows_core::RuntimeType for IMidiMessage64Factory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage64Factory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance3: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        *const u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage64Statics,
    IMidiMessage64Statics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000040064
);
impl windows_core::RuntimeType for IMidiMessage64Statics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage64Statics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        MidiMessageStruct,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage96,
    IMidiMessage96_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000040096
);
impl windows_core::RuntimeType for IMidiMessage96 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage96_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Word0: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord0: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word1: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord1: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Word2: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub SetWord2: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage96Factory,
    IMidiMessage96Factory_Vtbl,
    0x4ac4fe62_41c9_5605_9db8_ded7cb44b859
);
impl windows_core::RuntimeType for IMidiMessage96Factory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage96Factory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        u32,
        u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance3: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        u32,
        *const u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessage96Statics,
    IMidiMessage96Statics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000040096
);
impl windows_core::RuntimeType for IMidiMessage96Statics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessage96Statics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateFromStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u64,
        MidiMessageStruct,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessageReceivedEventArgs,
    IMidiMessageReceivedEventArgs_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000070000
);
impl windows_core::RuntimeType for IMidiMessageReceivedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiMessageReceivedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Timestamp:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u64) -> windows_core::HRESULT,
    pub PacketType: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut MidiPacketType,
    ) -> windows_core::HRESULT,
    MessageType: usize,
    pub PeekFirstWord:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub GetMessagePacket: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub FillWords: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut u32,
        *mut u32,
        *mut u32,
        *mut u32,
        *mut u8,
    ) -> windows_core::HRESULT,
    pub FillMessageStruct: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut MidiMessageStruct,
        *mut u8,
    ) -> windows_core::HRESULT,
    pub FillMessage32: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub FillMessage64: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub FillMessage96: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub FillMessage128: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub FillWordArray: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u32,
        u32,
        *mut u32,
        *mut u8,
    ) -> windows_core::HRESULT,
    pub FillByteArray: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        u32,
        u32,
        *mut u8,
        *mut u8,
    ) -> windows_core::HRESULT,
    FillBuffer: usize,
    pub AppendWordsToList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut u8,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiMessageReceivedEventSource,
    IMidiMessageReceivedEventSource_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000050
);
impl windows_core::RuntimeType for IMidiMessageReceivedEventSource {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiMessageReceivedEventSource,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiMessageReceivedEventSource {
    pub fn RemoveMessageReceived(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveMessageReceived)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn GetEndpointConnectionSource(
        &self,
    ) -> windows_core::Result<IMidiEndpointConnectionSource> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetEndpointConnectionSource)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
}
impl windows_core::RuntimeName for IMidiMessageReceivedEventSource {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.IMidiMessageReceivedEventSource";
}
pub trait IMidiMessageReceivedEventSource_Impl: windows_core::IUnknownImpl {
    fn RemoveMessageReceived(&self, token: i64) -> windows_core::Result<()>;
    fn GetEndpointConnectionSource(&self) -> windows_core::Result<IMidiEndpointConnectionSource>;
}
impl IMidiMessageReceivedEventSource_Vtbl {
    pub const fn new<Identity: IMidiMessageReceivedEventSource_Impl, const OFFSET: isize>() -> Self
    {
        unsafe extern "system" fn RemoveMessageReceived<
            Identity: IMidiMessageReceivedEventSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            token: i64,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiMessageReceivedEventSource_Impl::RemoveMessageReceived(this, token).into()
            }
        }
        unsafe extern "system" fn GetEndpointConnectionSource<
            Identity: IMidiMessageReceivedEventSource_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiMessageReceivedEventSource_Impl::GetEndpointConnectionSource(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiMessageReceivedEventSource,
                OFFSET,
            >(),
            MessageReceived: 0,
            RemoveMessageReceived: RemoveMessageReceived::<Identity, OFFSET>,
            GetEndpointConnectionSource: GetEndpointConnectionSource::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiMessageReceivedEventSource as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiMessageReceivedEventSource_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    MessageReceived: usize,
    pub RemoveMessageReceived:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub GetEndpointConnectionSource: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiServiceTransportPluginConfig,
    IMidiServiceTransportPluginConfig_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000060
);
impl windows_core::RuntimeType for IMidiServiceTransportPluginConfig {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiServiceTransportPluginConfig,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiServiceTransportPluginConfig {
    pub fn TransportId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).TransportId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeName for IMidiServiceTransportPluginConfig {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.ServiceConfig.IMidiServiceTransportPluginConfig";
}
pub trait IMidiServiceTransportPluginConfig_Impl: windows_core::IUnknownImpl {
    fn TransportId(&self) -> windows_core::Result<windows_core::GUID>;
}
impl IMidiServiceTransportPluginConfig_Vtbl {
    pub const fn new<Identity: IMidiServiceTransportPluginConfig_Impl, const OFFSET: isize>() -> Self
    {
        unsafe extern "system" fn TransportId<
            Identity: IMidiServiceTransportPluginConfig_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut windows_core::GUID,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiServiceTransportPluginConfig_Impl::TransportId(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<
                Identity,
                IMidiServiceTransportPluginConfig,
                OFFSET,
            >(),
            TransportId: TransportId::<Identity, OFFSET>,
            GetConfigJson: 0,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiServiceTransportPluginConfig as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiServiceTransportPluginConfig_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub TransportId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    GetConfigJson: usize,
}
windows_core::imp::define_interface!(
    IMidiRepository,
    IMidiRepository_Vtbl,
    0x8087b303_0519_31d1_c0de_dd0000080000
);
impl windows_core::RuntimeType for IMidiRepository {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiRepository_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub RepositoryId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub Name: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub IsOpen:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub Connections: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateEndpointConnection: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateEndpointConnection2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub DisconnectEndpointConnection: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub UpdateName: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiRepositoryStatics,
    IMidiRepositoryStatics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee0000080000
);
impl windows_core::RuntimeType for IMidiRepositoryStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiRepositoryStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Create: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiUniversalPacket,
    IMidiUniversalPacket_Vtbl,
    0x8087b303_0519_31d1_c0de_ff0000000010
);
impl windows_core::RuntimeType for IMidiUniversalPacket {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IMidiUniversalPacket,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IMidiUniversalPacket {
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetTimestamp(&self, value: u64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetTimestamp)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAllWords)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AppendAllMessageWordsToList<P0>(&self, targetlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendAllMessageWordsToList)(
                windows_core::Interface::as_raw(this),
                targetlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeName for IMidiUniversalPacket {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.IMidiUniversalPacket";
}
pub trait IMidiUniversalPacket_Impl: windows_core::IUnknownImpl {
    fn Timestamp(&self) -> windows_core::Result<u64>;
    fn SetTimestamp(&self, value: u64) -> windows_core::Result<()>;
    fn PacketType(&self) -> windows_core::Result<MidiPacketType>;
    fn PeekFirstWord(&self) -> windows_core::Result<u32>;
    fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>>;
    fn AppendAllMessageWordsToList(
        &self,
        targetList: windows_core::Ref<'_, windows_collections::IVector<u32>>,
    ) -> windows_core::Result<u8>;
}
impl IMidiUniversalPacket_Vtbl {
    pub const fn new<Identity: IMidiUniversalPacket_Impl, const OFFSET: isize>() -> Self {
        unsafe extern "system" fn Timestamp<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut u64,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiUniversalPacket_Impl::Timestamp(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn SetTimestamp<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            value: u64,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                IMidiUniversalPacket_Impl::SetTimestamp(this, value).into()
            }
        }
        unsafe extern "system" fn PacketType<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut MidiPacketType,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiUniversalPacket_Impl::PacketType(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn PeekFirstWord<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut u32,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiUniversalPacket_Impl::PeekFirstWord(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn GetAllWords<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiUniversalPacket_Impl::GetAllWords(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        unsafe extern "system" fn AppendAllMessageWordsToList<
            Identity: IMidiUniversalPacket_Impl,
            const OFFSET: isize,
        >(
            this: *mut core::ffi::c_void,
            targetlist: *mut core::ffi::c_void,
            result__: *mut u8,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IMidiUniversalPacket_Impl::AppendAllMessageWordsToList(
                    this,
                    core::mem::transmute_copy(&targetlist),
                ) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<Identity, IMidiUniversalPacket, OFFSET>(
            ),
            Timestamp: Timestamp::<Identity, OFFSET>,
            SetTimestamp: SetTimestamp::<Identity, OFFSET>,
            MessageType: 0,
            SetMessageType: 0,
            PacketType: PacketType::<Identity, OFFSET>,
            PeekFirstWord: PeekFirstWord::<Identity, OFFSET>,
            GetAllWords: GetAllWords::<Identity, OFFSET>,
            AppendAllMessageWordsToList: AppendAllMessageWordsToList::<Identity, OFFSET>,
            FillBuffer: 0,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IMidiUniversalPacket as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IMidiUniversalPacket_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Timestamp:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u64) -> windows_core::HRESULT,
    pub SetTimestamp:
        unsafe extern "system" fn(*mut core::ffi::c_void, u64) -> windows_core::HRESULT,
    MessageType: usize,
    SetMessageType: usize,
    pub PacketType: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut MidiPacketType,
    ) -> windows_core::HRESULT,
    pub PeekFirstWord:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    pub GetAllWords: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub AppendAllMessageWordsToList: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut u8,
    ) -> windows_core::HRESULT,
    FillBuffer: usize,
}
windows_core::imp::define_interface!(
    IMidiVirtualDevice,
    IMidiVirtualDevice_Vtbl,
    0x8087b303_0519_31d1_c0de_dd00002b0000
);
impl windows_core::RuntimeType for IMidiVirtualDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiVirtualDevice_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub DeviceEndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub AssociationId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    FunctionBlocks: usize,
    UpdateFunctionBlock: usize,
    pub UpdateEndpointName: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub SuppressHandledMessages:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetSuppressHandledMessages:
        unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    StreamConfigRequestReceived: usize,
    pub RemoveStreamConfigRequestReceived:
        unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiVirtualDeviceCreationConfig,
    IMidiVirtualDeviceCreationConfig_Vtbl,
    0x8087b303_0519_31d1_c0de_dd00002c0000
);
impl windows_core::RuntimeType for IMidiVirtualDeviceCreationConfig {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiVirtualDeviceCreationConfig_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Name: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetName: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Description: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetDescription: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub Manufacturer: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub SetManufacturer: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateOnlyUmpEndpoints:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetCreateOnlyUmpEndpoints:
        unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    pub AssociationId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub DeclaredDeviceIdentity: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut MidiDeclaredDeviceIdentity,
    ) -> windows_core::HRESULT,
    pub SetDeclaredDeviceIdentity: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        MidiDeclaredDeviceIdentity,
    ) -> windows_core::HRESULT,
    pub DeclaredEndpointInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
    ) -> windows_core::HRESULT,
    pub SetDeclaredEndpointInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
    ) -> windows_core::HRESULT,
    pub UserSuppliedInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::mem::MaybeUninit<MidiEndpointUserSuppliedInfo>,
    ) -> windows_core::HRESULT,
    pub SetUserSuppliedInfo: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        core::mem::MaybeUninit<MidiEndpointUserSuppliedInfo>,
    ) -> windows_core::HRESULT,
    FunctionBlocks: usize,
}
windows_core::imp::define_interface!(
    IMidiVirtualDeviceCreationConfigFactory,
    IMidiVirtualDeviceCreationConfigFactory_Vtbl,
    0x6b3bfe63_5c8f_57d8_8cba_208c938f0834
);
impl windows_core::RuntimeType for IMidiVirtualDeviceCreationConfigFactory {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiVirtualDeviceCreationConfigFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance2: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
        MidiDeclaredDeviceIdentity,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub CreateInstance3: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        core::mem::MaybeUninit<MidiDeclaredEndpointInfo>,
        MidiDeclaredDeviceIdentity,
        core::mem::MaybeUninit<MidiEndpointUserSuppliedInfo>,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IMidiVirtualDeviceManagerStatics,
    IMidiVirtualDeviceManagerStatics_Vtbl,
    0x8087b303_0519_31d1_c0de_ee00002d0000
);
impl windows_core::RuntimeType for IMidiVirtualDeviceManagerStatics {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMidiVirtualDeviceManagerStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub IsTransportAvailable:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub TransportId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut windows_core::GUID,
    ) -> windows_core::HRESULT,
    pub CreateVirtualDevice: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    pub GetAssociatedClientEndpointDeviceId: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        windows_core::GUID,
        *mut *mut core::ffi::c_void,
    )
        -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IStringable,
    IStringable_Vtbl,
    0x96369f54_8eb6_48f0_abce_c1b211e627c3
);
impl windows_core::RuntimeType for IStringable {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(
    IStringable,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl IStringable {
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
}
impl windows_core::RuntimeName for IStringable {
    const NAME: &'static str = "Windows.Foundation.IStringable";
}
pub trait IStringable_Impl: windows_core::IUnknownImpl {
    fn ToString(&self) -> windows_core::Result<windows_core::HSTRING>;
}
impl IStringable_Vtbl {
    pub const fn new<Identity: IStringable_Impl, const OFFSET: isize>() -> Self {
        unsafe extern "system" fn ToString<Identity: IStringable_Impl, const OFFSET: isize>(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity =
                    &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IStringable_Impl::ToString(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<Identity, IStringable, OFFSET>(),
            ToString: ToString::<Identity, OFFSET>,
        }
    }
    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IStringable as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IStringable_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub ToString: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MidiDeclaredDeviceIdentity {
    pub SystemExclusiveIdByte1: u8,
    pub SystemExclusiveIdByte2: u8,
    pub SystemExclusiveIdByte3: u8,
    pub DeviceFamilyLsb: u8,
    pub DeviceFamilyMsb: u8,
    pub DeviceFamilyModelNumberLsb: u8,
    pub DeviceFamilyModelNumberMsb: u8,
    pub SoftwareRevisionLevelByte1: u8,
    pub SoftwareRevisionLevelByte2: u8,
    pub SoftwareRevisionLevelByte3: u8,
    pub SoftwareRevisionLevelByte4: u8,
}
impl windows_core::TypeKind for MidiDeclaredDeviceIdentity {
    type TypeKind = windows_core::CopyType;
}
impl windows_core::RuntimeType for MidiDeclaredDeviceIdentity {
    const SIGNATURE :windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice ( b"struct(Microsoft.Windows.Devices.Midi2.MidiDeclaredDeviceIdentity;u1;u1;u1;u1;u1;u1;u1;u1;u1;u1;u1)" ) ;
}
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MidiDeclaredEndpointInfo {
    pub Name: windows_core::HSTRING,
    pub ProductInstanceId: windows_core::HSTRING,
    pub SupportsMidi10Protocol: bool,
    pub SupportsMidi20Protocol: bool,
    pub SupportsReceivingJitterReductionTimestamps: bool,
    pub SupportsSendingJitterReductionTimestamps: bool,
    pub HasStaticFunctionBlocks: bool,
    pub DeclaredFunctionBlockCount: u8,
    pub SpecificationVersionMajor: u8,
    pub SpecificationVersionMinor: u8,
}
impl windows_core::TypeKind for MidiDeclaredEndpointInfo {
    type TypeKind = windows_core::CloneType;
}
impl windows_core::RuntimeType for MidiDeclaredEndpointInfo {
    const SIGNATURE :windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice ( b"struct(Microsoft.Windows.Devices.Midi2.MidiDeclaredEndpointInfo;string;string;b1;b1;b1;b1;b1;u1;u1;u1)" ) ;
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointConnection(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointConnection,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(
    MidiEndpointConnection,
    IMidiEndpointConnectionSource,
    IMidiMessageReceivedEventSource,
    IStringable
);
impl MidiEndpointConnection {
    pub fn LogMessageDataValidationErrorDetails(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).LogMessageDataValidationErrorDetails)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetLogMessageDataValidationErrorDetails(&self, value: bool) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetLogMessageDataValidationErrorDetails)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Open(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Open)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn MessageProcessingPlugins(
        &self,
    ) -> windows_core::Result<windows_collections::IVectorView<IMidiEndpointMessageProcessingPlugin>>
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).MessageProcessingPlugins)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AddMessageProcessingPlugin<P0>(&self, plugin: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<IMidiEndpointMessageProcessingPlugin>,
    {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).AddMessageProcessingPlugin)(
                windows_core::Interface::as_raw(this),
                plugin.param().abi(),
            )
            .ok()
        }
    }
    pub fn RemoveMessageProcessingPlugin(
        &self,
        id: windows_core::GUID,
    ) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveMessageProcessingPlugin)(
                windows_core::Interface::as_raw(this),
                id,
            )
            .ok()
        }
    }
    pub fn SendSingleMessagePacket<P0>(
        &self,
        message: P0,
    ) -> windows_core::Result<MidiSendMessageResults>
    where
        P0: windows_core::Param<IMidiUniversalPacket>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessagePacket)(
                windows_core::Interface::as_raw(this),
                message.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageStruct(
        &self,
        timestamp: u64,
        wordcount: u8,
        message: MidiMessageStruct,
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageStruct)(
                windows_core::Interface::as_raw(this),
                timestamp,
                wordcount,
                &message,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageWordArray(
        &self,
        timestamp: u64,
        startindex: u32,
        wordcount: u8,
        words: &[u32],
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageWordArray)(
                windows_core::Interface::as_raw(this),
                timestamp,
                startindex,
                wordcount,
                words.len().try_into().unwrap(),
                words.as_ptr(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageWords(
        &self,
        timestamp: u64,
        word0: u32,
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageWords)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageWords2(
        &self,
        timestamp: u64,
        word0: u32,
        word1: u32,
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageWords2)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageWords3(
        &self,
        timestamp: u64,
        word0: u32,
        word1: u32,
        word2: u32,
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageWords3)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                word2,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendSingleMessageWords4(
        &self,
        timestamp: u64,
        word0: u32,
        word1: u32,
        word2: u32,
        word3: u32,
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendSingleMessageWords4)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                word2,
                word3,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendMultipleMessagesWordList<P1>(
        &self,
        timestamp: u64,
        words: P1,
    ) -> windows_core::Result<MidiSendMessageResults>
    where
        P1: windows_core::Param<windows_collections::IIterable<u32>>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMultipleMessagesWordList)(
                windows_core::Interface::as_raw(this),
                timestamp,
                words.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendMultipleMessagesWordArray(
        &self,
        timestamp: u64,
        startindex: u32,
        wordcount: u32,
        words: &[u32],
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMultipleMessagesWordArray)(
                windows_core::Interface::as_raw(this),
                timestamp,
                startindex,
                wordcount,
                words.len().try_into().unwrap(),
                words.as_ptr(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendMultipleMessagesPacketList<P0>(
        &self,
        messages: P0,
    ) -> windows_core::Result<MidiSendMessageResults>
    where
        P0: windows_core::Param<windows_collections::IIterable<IMidiUniversalPacket>>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMultipleMessagesPacketList)(
                windows_core::Interface::as_raw(this),
                messages.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendMultipleMessagesStructList<P1>(
        &self,
        timestamp: u64,
        messages: P1,
    ) -> windows_core::Result<MidiSendMessageResults>
    where
        P1: windows_core::Param<windows_collections::IIterable<MidiMessageStruct>>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMultipleMessagesStructList)(
                windows_core::Interface::as_raw(this),
                timestamp,
                messages.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SendMultipleMessagesStructArray(
        &self,
        timestamp: u64,
        startindex: u32,
        messagecount: u32,
        messages: &[MidiMessageStruct],
    ) -> windows_core::Result<MidiSendMessageResults> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMultipleMessagesStructArray)(
                windows_core::Interface::as_raw(this),
                timestamp,
                startindex,
                messagecount,
                messages.len().try_into().unwrap(),
                messages.as_ptr(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetSupportedMaxMidiWordsPerTransmission(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetSupportedMaxMidiWordsPerTransmission)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn RemoveEndpointDeviceDisconnected(&self, token: i64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveEndpointDeviceDisconnected)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveEndpointDeviceReconnected(&self, token: i64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveEndpointDeviceReconnected)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetTag)(
                windows_core::Interface::as_raw(this),
                value.param().abi(),
            )
            .ok()
        }
    }
    pub fn ConnectionId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ConnectionId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ConnectedEndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ConnectedEndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn Settings(&self) -> windows_core::Result<IMidiEndpointConnectionSettings> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Settings)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn IsOpen(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsOpen)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetDeviceSelector() -> windows_core::Result<windows_core::HSTRING> {
        Self::IMidiEndpointConnectionStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeviceSelector)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        })
    }
    pub fn SendMessageSucceeded(sendresult: MidiSendMessageResults) -> windows_core::Result<bool> {
        Self::IMidiEndpointConnectionStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMessageSucceeded)(
                windows_core::Interface::as_raw(this),
                sendresult,
                &mut result__,
            )
            .map(|| result__)
        })
    }
    pub fn SendMessageFailed(sendresult: MidiSendMessageResults) -> windows_core::Result<bool> {
        Self::IMidiEndpointConnectionStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SendMessageFailed)(
                windows_core::Interface::as_raw(this),
                sendresult,
                &mut result__,
            )
            .map(|| result__)
        })
    }
    pub fn RemoveMessageReceived(&self, token: i64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiMessageReceivedEventSource>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveMessageReceived)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn GetEndpointConnectionSource(
        &self,
    ) -> windows_core::Result<IMidiEndpointConnectionSource> {
        let this = &windows_core::Interface::cast::<IMidiMessageReceivedEventSource>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetEndpointConnectionSource)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiEndpointConnectionStatics<
        R,
        F: FnOnce(&IMidiEndpointConnectionStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiEndpointConnection,
            IMidiEndpointConnectionStatics,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiEndpointConnection {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiEndpointConnection>();
}
unsafe impl windows_core::Interface for MidiEndpointConnection {
    type Vtable = <IMidiEndpointConnection as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiEndpointConnection as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointConnection {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiEndpointConnection";
}
unsafe impl Send for MidiEndpointConnection {}
unsafe impl Sync for MidiEndpointConnection {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointConnectionBasicSettings(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointConnectionBasicSettings,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(
    MidiEndpointConnectionBasicSettings,
    IMidiEndpointConnectionSettings
);
impl MidiEndpointConnectionBasicSettings {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }
    fn IActivationFactory<
        R,
        F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiEndpointConnectionBasicSettings,
            windows_core::imp::IGenericFactory,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    pub fn CreateInstance(
        waitforendpointreceiptonsend: bool,
    ) -> windows_core::Result<MidiEndpointConnectionBasicSettings> {
        Self::IMidiEndpointConnectionBasicSettingsFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                waitforendpointreceiptonsend,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(
        waitforendpointreceiptonsend: bool,
        autoreconnect: bool,
    ) -> windows_core::Result<MidiEndpointConnectionBasicSettings> {
        Self::IMidiEndpointConnectionBasicSettingsFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                waitforendpointreceiptonsend,
                autoreconnect,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn WaitForEndpointReceiptOnSend(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSettings>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).WaitForEndpointReceiptOnSend)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AutoReconnect(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSettings>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AutoReconnect)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SettingsJson(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IMidiEndpointConnectionSettings>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SettingsJson)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiEndpointConnectionBasicSettingsFactory<
        R,
        F: FnOnce(&IMidiEndpointConnectionBasicSettingsFactory) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiEndpointConnectionBasicSettings,
            IMidiEndpointConnectionBasicSettingsFactory,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiEndpointConnectionBasicSettings {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiEndpointConnectionBasicSettings>();
}
unsafe impl windows_core::Interface for MidiEndpointConnectionBasicSettings {
    type Vtable = <IMidiEndpointConnectionBasicSettings as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiEndpointConnectionBasicSettings as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointConnectionBasicSettings {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.MidiEndpointConnectionBasicSettings";
}
unsafe impl Send for MidiEndpointConnectionBasicSettings {}
unsafe impl Sync for MidiEndpointConnectionBasicSettings {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointDeviceInformation(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointDeviceInformation,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(
    MidiEndpointDeviceInformation,
    IMidiEndpointDeviceInformation2,
    IStringable
);
impl MidiEndpointDeviceInformation {
    pub fn EndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).EndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn Name(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Name)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn ContainerId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ContainerId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn DeviceInstanceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).DeviceInstanceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn GetDeclaredEndpointInfo(&self) -> windows_core::Result<MidiDeclaredEndpointInfo> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeclaredEndpointInfo)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn GetDeclaredDeviceIdentity(&self) -> windows_core::Result<MidiDeclaredDeviceIdentity> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetDeclaredDeviceIdentity)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetUserSuppliedInfo(&self) -> windows_core::Result<MidiEndpointUserSuppliedInfo> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetUserSuppliedInfo)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn Properties(
        &self,
    ) -> windows_core::Result<
        windows_collections::IMapView<windows_core::HSTRING, windows_core::IInspectable>,
    > {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Properties)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn IsMuted(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointDeviceInformation2>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsMuted)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn CreateFromEndpointDeviceId(
        endpointdeviceid: &windows_core::HSTRING,
    ) -> windows_core::Result<MidiEndpointDeviceInformation> {
        Self::IMidiEndpointDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromEndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(endpointdeviceid),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn FindAll()
    -> windows_core::Result<windows_collections::IVectorView<MidiEndpointDeviceInformation>> {
        Self::IMidiEndpointDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FindAll)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateFromAssociatedMidi1PortDeviceId(
        deviceid: &windows_core::HSTRING,
    ) -> windows_core::Result<MidiEndpointDeviceInformation> {
        Self::IMidiEndpointDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromAssociatedMidi1PortDeviceId)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(deviceid),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn EndpointInterfaceClass() -> windows_core::Result<windows_core::GUID> {
        Self::IMidiEndpointDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).EndpointInterfaceClass)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        })
    }
    pub fn GetAdditionalPropertiesList()
    -> windows_core::Result<windows_collections::IVectorView<windows_core::HSTRING>> {
        Self::IMidiEndpointDeviceInformationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAdditionalPropertiesList)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiEndpointDeviceInformationStatics<
        R,
        F: FnOnce(&IMidiEndpointDeviceInformationStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiEndpointDeviceInformation,
            IMidiEndpointDeviceInformationStatics,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiEndpointDeviceInformation {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiEndpointDeviceInformation>();
}
unsafe impl windows_core::Interface for MidiEndpointDeviceInformation {
    type Vtable = <IMidiEndpointDeviceInformation as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiEndpointDeviceInformation as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointDeviceInformation {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiEndpointDeviceInformation";
}
unsafe impl Send for MidiEndpointDeviceInformation {}
unsafe impl Sync for MidiEndpointDeviceInformation {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointDeviceInformationAddedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointDeviceInformationAddedEventArgs,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl MidiEndpointDeviceInformationAddedEventArgs {
    pub fn AddedDevice(&self) -> windows_core::Result<MidiEndpointDeviceInformation> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AddedDevice)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
}
impl windows_core::RuntimeType for MidiEndpointDeviceInformationAddedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<
        Self,
        IMidiEndpointDeviceInformationAddedEventArgs,
    >();
}
unsafe impl windows_core::Interface for MidiEndpointDeviceInformationAddedEventArgs {
    type Vtable = <IMidiEndpointDeviceInformationAddedEventArgs as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiEndpointDeviceInformationAddedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointDeviceInformationAddedEventArgs {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.MidiEndpointDeviceInformationAddedEventArgs";
}
unsafe impl Send for MidiEndpointDeviceInformationAddedEventArgs {}
unsafe impl Sync for MidiEndpointDeviceInformationAddedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointDeviceInformationRemovedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointDeviceInformationRemovedEventArgs,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl MidiEndpointDeviceInformationRemovedEventArgs {
    pub fn EndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).EndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
}
impl windows_core::RuntimeType for MidiEndpointDeviceInformationRemovedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<
        Self,
        IMidiEndpointDeviceInformationRemovedEventArgs,
    >();
}
unsafe impl windows_core::Interface for MidiEndpointDeviceInformationRemovedEventArgs {
    type Vtable =
        <IMidiEndpointDeviceInformationRemovedEventArgs as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiEndpointDeviceInformationRemovedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointDeviceInformationRemovedEventArgs {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.MidiEndpointDeviceInformationRemovedEventArgs";
}
unsafe impl Send for MidiEndpointDeviceInformationRemovedEventArgs {}
unsafe impl Sync for MidiEndpointDeviceInformationRemovedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointDeviceInformationUpdatedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointDeviceInformationUpdatedEventArgs,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(
    MidiEndpointDeviceInformationUpdatedEventArgs,
    IMidiEndpointDeviceInformationUpdatedEventArgs2
);
impl MidiEndpointDeviceInformationUpdatedEventArgs {
    pub fn EndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).EndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn IsNameUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsNameUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn IsEndpointInformationUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsEndpointInformationUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn IsDeviceIdentityUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsDeviceIdentityUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn IsStreamConfigurationUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsStreamConfigurationUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AreFunctionBlocksUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AreFunctionBlocksUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn IsUserMetadataUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsUserMetadataUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AreAdditionalCapabilitiesUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AreAdditionalCapabilitiesUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AreUniqueIdsUpdated(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AreUniqueIdsUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AreGroupTerminalBlocksUpdated(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointDeviceInformationUpdatedEventArgs2>(
            self,
        )?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AreGroupTerminalBlocksUpdated)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for MidiEndpointDeviceInformationUpdatedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<
        Self,
        IMidiEndpointDeviceInformationUpdatedEventArgs,
    >();
}
unsafe impl windows_core::Interface for MidiEndpointDeviceInformationUpdatedEventArgs {
    type Vtable =
        <IMidiEndpointDeviceInformationUpdatedEventArgs as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiEndpointDeviceInformationUpdatedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointDeviceInformationUpdatedEventArgs {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.MidiEndpointDeviceInformationUpdatedEventArgs";
}
unsafe impl Send for MidiEndpointDeviceInformationUpdatedEventArgs {}
unsafe impl Sync for MidiEndpointDeviceInformationUpdatedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiEndpointDeviceWatcher(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiEndpointDeviceWatcher,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl MidiEndpointDeviceWatcher {
    pub fn Start(&self) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).Start)(windows_core::Interface::as_raw(this))
                .ok()
        }
    }
    pub fn Stop(&self) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).Stop)(windows_core::Interface::as_raw(this)).ok()
        }
    }
    pub fn EnumeratedEndpointDevices(
        &self,
    ) -> windows_core::Result<
        windows_collections::IMapView<windows_core::HSTRING, MidiEndpointDeviceInformation>,
    > {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).EnumeratedEndpointDevices)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn RemoveAdded(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveAdded)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveRemoved(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveRemoved)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveUpdated(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveUpdated)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveEnumerationCompleted(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveEnumerationCompleted)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn RemoveStopped(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveStopped)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
    pub fn Create() -> windows_core::Result<MidiEndpointDeviceWatcher> {
        Self::IMidiEndpointDeviceWatcherStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Create)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    fn IMidiEndpointDeviceWatcherStatics<
        R,
        F: FnOnce(&IMidiEndpointDeviceWatcherStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiEndpointDeviceWatcher,
            IMidiEndpointDeviceWatcherStatics,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiEndpointDeviceWatcher {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiEndpointDeviceWatcher>();
}
unsafe impl windows_core::Interface for MidiEndpointDeviceWatcher {
    type Vtable = <IMidiEndpointDeviceWatcher as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiEndpointDeviceWatcher as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiEndpointDeviceWatcher {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiEndpointDeviceWatcher";
}
unsafe impl Send for MidiEndpointDeviceWatcher {}
unsafe impl Sync for MidiEndpointDeviceWatcher {}
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MidiEndpointUserSuppliedInfo {
    pub Name: windows_core::HSTRING,
    pub Description: windows_core::HSTRING,
    pub ImageFileName: windows_core::HSTRING,
    pub RequiresNoteOffTranslation: bool,
    pub RecommendedControlChangeAutomationIntervalMilliseconds: u16,
    pub SupportsMidiPolyphonicExpression: bool,
}
impl windows_core::TypeKind for MidiEndpointUserSuppliedInfo {
    type TypeKind = windows_core::CloneType;
}
impl windows_core::RuntimeType for MidiEndpointUserSuppliedInfo {
    const SIGNATURE :windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice ( b"struct(Microsoft.Windows.Devices.Midi2.MidiEndpointUserSuppliedInfo;string;string;string;b1;u2;b1)" ) ;
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiMessage128(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiMessage128,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiMessage128, IMidiUniversalPacket, IStringable);
impl MidiMessage128 {
    pub fn Word0(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word0)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord0(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord0)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word1(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word1)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord1(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord1)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word2(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word2)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord2(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord2)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word3(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word3)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord3(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord3)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn new() -> windows_core::Result<MidiMessage128> {
        Self::IMidiMessage128Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(
        timestamp: u64,
        word0: u32,
        word1: u32,
        word2: u32,
        word3: u32,
    ) -> windows_core::Result<MidiMessage128> {
        Self::IMidiMessage128Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                word2,
                word3,
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance3(timestamp: u64, words: &[u32]) -> windows_core::Result<MidiMessage128> {
        Self::IMidiMessage128Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance3)(
                windows_core::Interface::as_raw(this),
                timestamp,
                words.len().try_into().unwrap(),
                words.as_ptr(),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateFromStruct(
        timestamp: u64,
        message: MidiMessageStruct,
    ) -> windows_core::Result<MidiMessage128> {
        Self::IMidiMessage128Statics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromStruct)(
                windows_core::Interface::as_raw(this),
                timestamp,
                message,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetTimestamp(&self, value: u64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetTimestamp)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAllWords)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AppendAllMessageWordsToList<P0>(&self, targetlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendAllMessageWordsToList)(
                windows_core::Interface::as_raw(this),
                targetlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiMessage128Factory<R, F: FnOnce(&IMidiMessage128Factory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage128, IMidiMessage128Factory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    fn IMidiMessage128Statics<R, F: FnOnce(&IMidiMessage128Statics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage128, IMidiMessage128Statics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiMessage128 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiMessage128>();
}
unsafe impl windows_core::Interface for MidiMessage128 {
    type Vtable = <IMidiMessage128 as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiMessage128 as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiMessage128 {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiMessage128";
}
unsafe impl Send for MidiMessage128 {}
unsafe impl Sync for MidiMessage128 {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiMessage32(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiMessage32,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiMessage32, IMidiUniversalPacket, IStringable);
impl MidiMessage32 {
    pub fn Word0(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word0)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord0(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord0)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn new() -> windows_core::Result<MidiMessage32> {
        Self::IMidiMessage32Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(timestamp: u64, word0: u32) -> windows_core::Result<MidiMessage32> {
        Self::IMidiMessage32Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateFromStruct(
        timestamp: u64,
        message: MidiMessageStruct,
    ) -> windows_core::Result<MidiMessage32> {
        Self::IMidiMessage32Statics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromStruct)(
                windows_core::Interface::as_raw(this),
                timestamp,
                message,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetTimestamp(&self, value: u64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetTimestamp)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAllWords)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AppendAllMessageWordsToList<P0>(&self, targetlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendAllMessageWordsToList)(
                windows_core::Interface::as_raw(this),
                targetlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiMessage32Factory<R, F: FnOnce(&IMidiMessage32Factory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage32, IMidiMessage32Factory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    fn IMidiMessage32Statics<R, F: FnOnce(&IMidiMessage32Statics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage32, IMidiMessage32Statics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiMessage32 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiMessage32>();
}
unsafe impl windows_core::Interface for MidiMessage32 {
    type Vtable = <IMidiMessage32 as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiMessage32 as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiMessage32 {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiMessage32";
}
unsafe impl Send for MidiMessage32 {}
unsafe impl Sync for MidiMessage32 {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiMessage64(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiMessage64,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiMessage64, IMidiUniversalPacket, IStringable);
impl MidiMessage64 {
    pub fn Word0(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word0)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord0(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord0)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word1(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word1)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord1(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord1)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn new() -> windows_core::Result<MidiMessage64> {
        Self::IMidiMessage64Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(
        timestamp: u64,
        word0: u32,
        word1: u32,
    ) -> windows_core::Result<MidiMessage64> {
        Self::IMidiMessage64Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance3(timestamp: u64, words: &[u32]) -> windows_core::Result<MidiMessage64> {
        Self::IMidiMessage64Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance3)(
                windows_core::Interface::as_raw(this),
                timestamp,
                words.len().try_into().unwrap(),
                words.as_ptr(),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateFromStruct(
        timestamp: u64,
        message: MidiMessageStruct,
    ) -> windows_core::Result<MidiMessage64> {
        Self::IMidiMessage64Statics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromStruct)(
                windows_core::Interface::as_raw(this),
                timestamp,
                message,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetTimestamp(&self, value: u64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetTimestamp)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAllWords)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AppendAllMessageWordsToList<P0>(&self, targetlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendAllMessageWordsToList)(
                windows_core::Interface::as_raw(this),
                targetlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiMessage64Factory<R, F: FnOnce(&IMidiMessage64Factory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage64, IMidiMessage64Factory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    fn IMidiMessage64Statics<R, F: FnOnce(&IMidiMessage64Statics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage64, IMidiMessage64Statics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiMessage64 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiMessage64>();
}
unsafe impl windows_core::Interface for MidiMessage64 {
    type Vtable = <IMidiMessage64 as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiMessage64 as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiMessage64 {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiMessage64";
}
unsafe impl Send for MidiMessage64 {}
unsafe impl Sync for MidiMessage64 {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiMessage96(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiMessage96,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiMessage96, IMidiUniversalPacket, IStringable);
impl MidiMessage96 {
    pub fn Word0(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word0)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord0(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord0)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word1(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word1)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord1(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord1)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Word2(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Word2)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetWord2(&self, value: u32) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetWord2)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn new() -> windows_core::Result<MidiMessage96> {
        Self::IMidiMessage96Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(
        timestamp: u64,
        word0: u32,
        word1: u32,
        word2: u32,
    ) -> windows_core::Result<MidiMessage96> {
        Self::IMidiMessage96Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                timestamp,
                word0,
                word1,
                word2,
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance3(timestamp: u64, words: &[u32]) -> windows_core::Result<MidiMessage96> {
        Self::IMidiMessage96Factory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance3)(
                windows_core::Interface::as_raw(this),
                timestamp,
                words.len().try_into().unwrap(),
                words.as_ptr(),
                core::ptr::null_mut(),
                &mut core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateFromStruct(
        timestamp: u64,
        message: MidiMessageStruct,
    ) -> windows_core::Result<MidiMessage96> {
        Self::IMidiMessage96Statics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateFromStruct)(
                windows_core::Interface::as_raw(this),
                timestamp,
                message,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetTimestamp(&self, value: u64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetTimestamp)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetAllWords(&self) -> windows_core::Result<windows_collections::IVector<u32>> {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAllWords)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn AppendAllMessageWordsToList<P0>(&self, targetlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = &windows_core::Interface::cast::<IMidiUniversalPacket>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendAllMessageWordsToList)(
                windows_core::Interface::as_raw(this),
                targetlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiMessage96Factory<R, F: FnOnce(&IMidiMessage96Factory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage96, IMidiMessage96Factory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
    fn IMidiMessage96Statics<R, F: FnOnce(&IMidiMessage96Statics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiMessage96, IMidiMessage96Statics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiMessage96 {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiMessage96>();
}
unsafe impl windows_core::Interface for MidiMessage96 {
    type Vtable = <IMidiMessage96 as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiMessage96 as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiMessage96 {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiMessage96";
}
unsafe impl Send for MidiMessage96 {}
unsafe impl Sync for MidiMessage96 {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiMessageReceivedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiMessageReceivedEventArgs,
    windows_core::IUnknown,
    windows_core::IInspectable
);
impl MidiMessageReceivedEventArgs {
    pub fn Timestamp(&self) -> windows_core::Result<u64> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Timestamp)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PacketType(&self) -> windows_core::Result<MidiPacketType> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PacketType)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PeekFirstWord(&self) -> windows_core::Result<u32> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PeekFirstWord)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn GetMessagePacket(&self) -> windows_core::Result<IMidiUniversalPacket> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetMessagePacket)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn FillWords(
        &self,
        word0: &mut u32,
        word1: &mut u32,
        word2: &mut u32,
        word3: &mut u32,
    ) -> windows_core::Result<u8> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillWords)(
                windows_core::Interface::as_raw(this),
                word0,
                word1,
                word2,
                word3,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillMessageStruct(&self, message: &mut MidiMessageStruct) -> windows_core::Result<u8> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillMessageStruct)(
                windows_core::Interface::as_raw(this),
                message,
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillMessage32<P0>(&self, message: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<MidiMessage32>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillMessage32)(
                windows_core::Interface::as_raw(this),
                message.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillMessage64<P0>(&self, message: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<MidiMessage64>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillMessage64)(
                windows_core::Interface::as_raw(this),
                message.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillMessage96<P0>(&self, message: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<MidiMessage96>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillMessage96)(
                windows_core::Interface::as_raw(this),
                message.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillMessage128<P0>(&self, message: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<MidiMessage128>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillMessage128)(
                windows_core::Interface::as_raw(this),
                message.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillWordArray(&self, startindex: u32, words: &mut [u32]) -> windows_core::Result<u8> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillWordArray)(
                windows_core::Interface::as_raw(this),
                startindex,
                words.len().try_into().unwrap(),
                words.as_mut_ptr(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn FillByteArray(&self, startindex: u32, bytes: &mut [u8]) -> windows_core::Result<u8> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).FillByteArray)(
                windows_core::Interface::as_raw(this),
                startindex,
                bytes.len().try_into().unwrap(),
                bytes.as_mut_ptr(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn AppendWordsToList<P0>(&self, wordlist: P0) -> windows_core::Result<u8>
    where
        P0: windows_core::Param<windows_collections::IVector<u32>>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppendWordsToList)(
                windows_core::Interface::as_raw(this),
                wordlist.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for MidiMessageReceivedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiMessageReceivedEventArgs>();
}
unsafe impl windows_core::Interface for MidiMessageReceivedEventArgs {
    type Vtable = <IMidiMessageReceivedEventArgs as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiMessageReceivedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiMessageReceivedEventArgs {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiMessageReceivedEventArgs";
}
unsafe impl Send for MidiMessageReceivedEventArgs {}
unsafe impl Sync for MidiMessageReceivedEventArgs {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MidiMessageStruct {
    pub Word0: u32,
    pub Word1: u32,
    pub Word2: u32,
    pub Word3: u32,
}
impl windows_core::TypeKind for MidiMessageStruct {
    type TypeKind = windows_core::CopyType;
}
impl windows_core::RuntimeType for MidiMessageStruct {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"struct(Microsoft.Windows.Devices.Midi2.MidiMessageStruct;u4;u4;u4;u4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidiPacketType(pub i32);
impl MidiPacketType {
    pub const UnknownOrInvalid: Self = Self(0i32);
    pub const UniversalMidiPacket32: Self = Self(1i32);
    pub const UniversalMidiPacket64: Self = Self(2i32);
    pub const UniversalMidiPacket96: Self = Self(3i32);
    pub const UniversalMidiPacket128: Self = Self(4i32);
}
impl windows_core::TypeKind for MidiPacketType {
    type TypeKind = windows_core::CopyType;
}
impl windows_core::RuntimeType for MidiPacketType {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Microsoft.Windows.Devices.Midi2.MidiPacketType;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidiProtocol(pub i32);
impl MidiProtocol {
    pub const Default: Self = Self(0i32);
    pub const Midi1: Self = Self(1i32);
    pub const Midi2: Self = Self(2i32);
}
impl windows_core::TypeKind for MidiProtocol {
    type TypeKind = windows_core::CopyType;
}
impl windows_core::RuntimeType for MidiProtocol {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Microsoft.Windows.Devices.Midi2.MidiProtocol;i4)",
    );
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidiSendMessageResults(pub u32);
impl MidiSendMessageResults {
    pub const Succeeded: Self = Self(2147483648u32);
    pub const Failed: Self = Self(268435456u32);
    pub const BufferFull: Self = Self(65536u32);
    pub const EndpointConnectionClosedOrInvalid: Self = Self(262144u32);
    pub const InvalidMessageTypeForWordCount: Self = Self(1048576u32);
    pub const InvalidMessageOther: Self = Self(2097152u32);
    pub const DataIndexOutOfRange: Self = Self(4194304u32);
    pub const TimestampOutOfRange: Self = Self(8388608u32);
    pub const TransmissionWordCountExceeded: Self = Self(16777216u32);
    pub const MessageListPartiallyProcessed: Self = Self(15728640u32);
}
impl windows_core::TypeKind for MidiSendMessageResults {
    type TypeKind = windows_core::CopyType;
}
impl windows_core::RuntimeType for MidiSendMessageResults {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(
        b"enum(Microsoft.Windows.Devices.Midi2.MidiSendMessageResults;u4)",
    );
}
impl MidiSendMessageResults {
    pub const fn contains(&self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
impl core::ops::BitOr for MidiSendMessageResults {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
impl core::ops::BitAnd for MidiSendMessageResults {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}
impl core::ops::BitOrAssign for MidiSendMessageResults {
    fn bitor_assign(&mut self, other: Self) {
        self.0.bitor_assign(other.0)
    }
}
impl core::ops::BitAndAssign for MidiSendMessageResults {
    fn bitand_assign(&mut self, other: Self) {
        self.0.bitand_assign(other.0)
    }
}
impl core::ops::Not for MidiSendMessageResults {
    type Output = Self;
    fn not(self) -> Self {
        Self(self.0.not())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiRepository(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiRepository,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiRepository, IClosable, IStringable);
impl MidiRepository {
    pub fn Close(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IClosable>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).Close)(windows_core::Interface::as_raw(this))
                .ok()
        }
    }
    pub fn RepositoryId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).RepositoryId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn Name(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Name)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn IsOpen(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsOpen)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn Connections(
        &self,
    ) -> windows_core::Result<
        windows_collections::IMapView<windows_core::GUID, MidiEndpointConnection>,
    > {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Connections)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn CreateEndpointConnection(
        &self,
        endpointdeviceid: &windows_core::HSTRING,
    ) -> windows_core::Result<MidiEndpointConnection> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateEndpointConnection)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(endpointdeviceid),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn CreateEndpointConnection2<P1>(
        &self,
        endpointdeviceid: &windows_core::HSTRING,
        settings: P1,
    ) -> windows_core::Result<MidiEndpointConnection>
    where
        P1: windows_core::Param<IMidiEndpointConnectionSettings>,
    {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateEndpointConnection2)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(endpointdeviceid),
                settings.param().abi(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn DisconnectEndpointConnection(
        &self,
        endpointconnectionid: windows_core::GUID,
    ) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).DisconnectEndpointConnection)(
                windows_core::Interface::as_raw(this),
                endpointconnectionid,
            )
            .ok()
        }
    }
    pub fn UpdateName(&self, newname: &windows_core::HSTRING) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).UpdateName)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(newname),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn Create(sessionname: &windows_core::HSTRING) -> windows_core::Result<MidiRepository> {
        Self::IMidiRepositoryStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Create)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(sessionname),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn ToString(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IStringable>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ToString)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    fn IMidiRepositoryStatics<R, F: FnOnce(&IMidiRepositoryStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MidiRepository, IMidiRepositoryStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiRepository {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiRepository>();
}
unsafe impl windows_core::Interface for MidiRepository {
    type Vtable = <IMidiRepository as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiRepository as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiRepository {
    const NAME: &'static str = "Microsoft.Windows.Devices.Midi2.MidiRepository";
}
unsafe impl Send for MidiRepository {}
unsafe impl Sync for MidiRepository {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiVirtualDevice(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiVirtualDevice,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(MidiVirtualDevice, IMidiEndpointMessageProcessingPlugin);
impl MidiVirtualDevice {
    pub fn PluginId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn PluginName(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginName)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetPluginName(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetPluginName)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn PluginTag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).PluginTag)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        }
    }
    pub fn SetPluginTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetPluginTag)(
                windows_core::Interface::as_raw(this),
                value.param().abi(),
            )
            .ok()
        }
    }
    pub fn IsEnabled(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsEnabled)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetIsEnabled)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn Initialize<P0>(&self, endpointconnection: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<IMidiEndpointConnectionSource>,
    {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).Initialize)(
                windows_core::Interface::as_raw(this),
                endpointconnection.param().abi(),
            )
            .ok()
        }
    }
    pub fn OnEndpointConnectionOpened(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).OnEndpointConnectionOpened)(
                windows_core::Interface::as_raw(this),
            )
            .ok()
        }
    }
    pub fn ProcessIncomingMessage<P0>(
        &self,
        args: P0,
        skipfurtherlisteners: &mut bool,
        skipmainmessagereceivedevent: &mut bool,
    ) -> windows_core::Result<()>
    where
        P0: windows_core::Param<MidiMessageReceivedEventArgs>,
    {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).ProcessIncomingMessage)(
                windows_core::Interface::as_raw(this),
                args.param().abi(),
                skipfurtherlisteners,
                skipmainmessagereceivedevent,
            )
            .ok()
        }
    }
    pub fn Cleanup(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IMidiEndpointMessageProcessingPlugin>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).Cleanup)(windows_core::Interface::as_raw(this))
                .ok()
        }
    }
    pub fn DeviceEndpointDeviceId(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).DeviceEndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn AssociationId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AssociationId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn UpdateEndpointName(&self, name: &windows_core::HSTRING) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).UpdateEndpointName)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(name),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SuppressHandledMessages(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SuppressHandledMessages)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetSuppressHandledMessages(&self, value: bool) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetSuppressHandledMessages)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn RemoveStreamConfigRequestReceived(&self, token: i64) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).RemoveStreamConfigRequestReceived)(
                windows_core::Interface::as_raw(this),
                token,
            )
            .ok()
        }
    }
}
impl windows_core::RuntimeType for MidiVirtualDevice {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiVirtualDevice>();
}
unsafe impl windows_core::Interface for MidiVirtualDevice {
    type Vtable = <IMidiVirtualDevice as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID = <IMidiVirtualDevice as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiVirtualDevice {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.Endpoints.Virtual.MidiVirtualDevice";
}
unsafe impl Send for MidiVirtualDevice {}
unsafe impl Sync for MidiVirtualDevice {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MidiVirtualDeviceCreationConfig(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    MidiVirtualDeviceCreationConfig,
    windows_core::IUnknown,
    windows_core::IInspectable
);
windows_core::imp::required_hierarchy!(
    MidiVirtualDeviceCreationConfig,
    IMidiServiceTransportPluginConfig
);
impl MidiVirtualDeviceCreationConfig {
    pub fn TransportId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = &windows_core::Interface::cast::<IMidiServiceTransportPluginConfig>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).TransportId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn Name(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Name)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetName(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetName)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn Description(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Description)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetDescription(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetDescription)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn Manufacturer(&self) -> windows_core::Result<windows_core::HSTRING> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Manufacturer)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetManufacturer(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetManufacturer)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn CreateOnlyUmpEndpoints(&self) -> windows_core::Result<bool> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateOnlyUmpEndpoints)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetCreateOnlyUmpEndpoints(&self, value: bool) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetCreateOnlyUmpEndpoints)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn AssociationId(&self) -> windows_core::Result<windows_core::GUID> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AssociationId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn DeclaredDeviceIdentity(&self) -> windows_core::Result<MidiDeclaredDeviceIdentity> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).DeclaredDeviceIdentity)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        }
    }
    pub fn SetDeclaredDeviceIdentity(
        &self,
        value: MidiDeclaredDeviceIdentity,
    ) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetDeclaredDeviceIdentity)(
                windows_core::Interface::as_raw(this),
                value,
            )
            .ok()
        }
    }
    pub fn DeclaredEndpointInfo(&self) -> windows_core::Result<MidiDeclaredEndpointInfo> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).DeclaredEndpointInfo)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetDeclaredEndpointInfo(
        &self,
        value: &MidiDeclaredEndpointInfo,
    ) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetDeclaredEndpointInfo)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn UserSuppliedInfo(&self) -> windows_core::Result<MidiEndpointUserSuppliedInfo> {
        let this = self;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).UserSuppliedInfo)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        }
    }
    pub fn SetUserSuppliedInfo(
        &self,
        value: &MidiEndpointUserSuppliedInfo,
    ) -> windows_core::Result<()> {
        let this = self;
        unsafe {
            (windows_core::Interface::vtable(this).SetUserSuppliedInfo)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }
    pub fn CreateInstance(
        name: &windows_core::HSTRING,
        description: &windows_core::HSTRING,
        manufacturer: &windows_core::HSTRING,
        declaredendpointinfo: &MidiDeclaredEndpointInfo,
    ) -> windows_core::Result<MidiVirtualDeviceCreationConfig> {
        Self::IMidiVirtualDeviceCreationConfigFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(name),
                core::mem::transmute_copy(description),
                core::mem::transmute_copy(manufacturer),
                core::mem::transmute_copy(declaredendpointinfo),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance2(
        name: &windows_core::HSTRING,
        description: &windows_core::HSTRING,
        manufacturer: &windows_core::HSTRING,
        declaredendpointinfo: &MidiDeclaredEndpointInfo,
        declareddeviceidentity: MidiDeclaredDeviceIdentity,
    ) -> windows_core::Result<MidiVirtualDeviceCreationConfig> {
        Self::IMidiVirtualDeviceCreationConfigFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance2)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(name),
                core::mem::transmute_copy(description),
                core::mem::transmute_copy(manufacturer),
                core::mem::transmute_copy(declaredendpointinfo),
                declareddeviceidentity,
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn CreateInstance3(
        name: &windows_core::HSTRING,
        description: &windows_core::HSTRING,
        manufacturer: &windows_core::HSTRING,
        declaredendpointinfo: &MidiDeclaredEndpointInfo,
        declareddeviceidentity: MidiDeclaredDeviceIdentity,
        usersuppliedinfo: &MidiEndpointUserSuppliedInfo,
    ) -> windows_core::Result<MidiVirtualDeviceCreationConfig> {
        Self::IMidiVirtualDeviceCreationConfigFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance3)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(name),
                core::mem::transmute_copy(description),
                core::mem::transmute_copy(manufacturer),
                core::mem::transmute_copy(declaredendpointinfo),
                declareddeviceidentity,
                core::mem::transmute_copy(usersuppliedinfo),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    fn IMidiVirtualDeviceCreationConfigFactory<
        R,
        F: FnOnce(&IMidiVirtualDeviceCreationConfigFactory) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiVirtualDeviceCreationConfig,
            IMidiVirtualDeviceCreationConfigFactory,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MidiVirtualDeviceCreationConfig {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IMidiVirtualDeviceCreationConfig>();
}
unsafe impl windows_core::Interface for MidiVirtualDeviceCreationConfig {
    type Vtable = <IMidiVirtualDeviceCreationConfig as windows_core::Interface>::Vtable;
    const IID: windows_core::GUID =
        <IMidiVirtualDeviceCreationConfig as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MidiVirtualDeviceCreationConfig {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.Endpoints.Virtual.MidiVirtualDeviceCreationConfig";
}
unsafe impl Send for MidiVirtualDeviceCreationConfig {}
unsafe impl Sync for MidiVirtualDeviceCreationConfig {}
pub struct MidiVirtualDeviceManager;
impl MidiVirtualDeviceManager {
    pub fn IsTransportAvailable() -> windows_core::Result<bool> {
        Self::IMidiVirtualDeviceManagerStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsTransportAvailable)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        })
    }
    pub fn TransportId() -> windows_core::Result<windows_core::GUID> {
        Self::IMidiVirtualDeviceManagerStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).TransportId)(
                windows_core::Interface::as_raw(this),
                &mut result__,
            )
            .map(|| result__)
        })
    }
    pub fn CreateVirtualDevice<P0>(creationconfig: P0) -> windows_core::Result<MidiVirtualDevice>
    where
        P0: windows_core::Param<MidiVirtualDeviceCreationConfig>,
    {
        Self::IMidiVirtualDeviceManagerStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateVirtualDevice)(
                windows_core::Interface::as_raw(this),
                creationconfig.param().abi(),
                &mut result__,
            )
            .and_then(|| windows_core::Type::from_abi(result__))
        })
    }
    pub fn GetAssociatedClientEndpointDeviceId(
        associationid: windows_core::GUID,
    ) -> windows_core::Result<windows_core::HSTRING> {
        Self::IMidiVirtualDeviceManagerStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetAssociatedClientEndpointDeviceId)(
                windows_core::Interface::as_raw(this),
                associationid,
                &mut result__,
            )
            .map(|| core::mem::transmute(result__))
        })
    }
    fn IMidiVirtualDeviceManagerStatics<
        R,
        F: FnOnce(&IMidiVirtualDeviceManagerStatics) -> windows_core::Result<R>,
    >(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<
            MidiVirtualDeviceManager,
            IMidiVirtualDeviceManagerStatics,
        > = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeName for MidiVirtualDeviceManager {
    const NAME: &'static str =
        "Microsoft.Windows.Devices.Midi2.Endpoints.Virtual.MidiVirtualDeviceManager";
}
