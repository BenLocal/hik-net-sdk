use std::{mem, os::raw::c_char};

use crate::{
    DWORD, LONG, LPNET_DVR_DEVICEINFO_V30, NET_DVR_DEVICEINFO_V30, NET_DVR_GET_IPPARACFG_V40,
    NET_DVR_GetDVRConfig, NET_DVR_IPPARACFG_V40, NET_DVR_Login_V30, NET_DVR_Logout_V30,
    as_c_string, common::get_last_error_code,
};

pub struct HikDevice {
    login_hanlder: Option<LONG>,
    device_info: Option<HikDeviceInfo>,
}

impl HikDevice {
    pub fn new() -> Self {
        Self {
            login_hanlder: None,
            device_info: None,
        }
    }

    pub fn login(
        &mut self,
        ip: &str,
        username: &str,
        password: &str,
        port: u16,
    ) -> anyhow::Result<&mut Self> {
        let ip = as_c_string!(ip);
        let username = as_c_string!(username);
        let password = as_c_string!(password);

        let mut device_info = NET_DVR_DEVICEINFO_V30::default();

        let res = unsafe {
            NET_DVR_Login_V30(
                ip.as_ptr() as *mut c_char,
                port,
                username.as_ptr() as *mut c_char,
                password.as_ptr() as *mut c_char,
                &mut device_info as LPNET_DVR_DEVICEINFO_V30,
            )
        };

        if res < 0 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!("Login failed: error code {}", error_code));
        }

        self.device_info = Some(HikDeviceInfo::new(device_info));
        self.login_hanlder = Some(res);
        Ok(self)
    }

    pub fn logout(&mut self) -> anyhow::Result<&mut Self> {
        if let Some(login_hanlder) = self.login_hanlder.take() {
            unsafe {
                NET_DVR_Logout_V30(login_hanlder);
            }
        }
        self.device_info = None;
        self.login_hanlder = None;
        Ok(self)
    }

    // pub fn get_device_config(&self) -> anyhow::Result<()> {
    //     let lu = self
    //         .login_hanlder
    //         .ok_or(anyhow::anyhow!("Login hanlder not found"))?;
    //     let res = unsafe {
    //         NET_DVR_GetDeviceConfig(
    //             lu,
    //             NET_DVR_GET_CHANNELINFO,
    //             i_group_no,
    //             &mut device_cfg_v40 as *mut _ as *mut std::ffi::c_void,
    //             size,
    //             &mut dw_returned,
    //         )
    //     };}

    // }

    pub fn get_ip_channel_config(&self) -> anyhow::Result<NET_DVR_IPPARACFG_V40> {
        let lu = self
            .login_hanlder
            .ok_or(anyhow::anyhow!("Login hanlder not found"))?;

        let mut ip_access_cfg_v40: NET_DVR_IPPARACFG_V40 = unsafe { mem::zeroed() };
        // iGroupNO = 0
        let i_group_no: LONG = 0;
        // 返回的大小
        let mut dw_returned: DWORD = 0;
        // 结构体大小
        let size = mem::size_of::<NET_DVR_IPPARACFG_V40>() as DWORD;

        let res = unsafe {
            NET_DVR_GetDVRConfig(
                lu,
                NET_DVR_GET_IPPARACFG_V40,
                i_group_no,
                &mut ip_access_cfg_v40 as *mut _ as *mut std::ffi::c_void,
                size,
                &mut dw_returned,
            )
        };

        if res == 0 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Get IP channel config failed: error code {}, dwReturned: {}",
                error_code,
                dw_returned
            ));
        }

        println!("ip_access_cfg_v40: {:?}", ip_access_cfg_v40.dwSize);
        println!("ip_access_cfg_v40: {:?}", ip_access_cfg_v40.dwGroupNum);
        println!("ip_access_cfg_v40: {:?}", ip_access_cfg_v40.dwAChanNum);
        println!("ip_access_cfg_v40: {:?}", ip_access_cfg_v40.dwDChanNum);
        println!("ip_access_cfg_v40: {:?}", ip_access_cfg_v40.dwStartDChan);

        Ok(ip_access_cfg_v40)
    }
}

impl Drop for HikDevice {
    fn drop(&mut self) {
        if let Some(login_hanlder) = self.login_hanlder {
            unsafe {
                NET_DVR_Logout_V30(login_hanlder);
            }
        }
    }
}

#[derive(Debug)]
pub enum Channel {
    Logic(ChannelInfo),
    IP(ChannelInfo),
}

#[derive(Debug)]
pub struct ChannelInfo {
    index: u16,
    chan_num: u16,
}

pub struct HikDeviceInfo(NET_DVR_DEVICEINFO_V30);

impl HikDeviceInfo {
    pub fn new(device_info: NET_DVR_DEVICEINFO_V30) -> Self {
        Self(device_info)
    }

    pub fn get_channels(&self) -> Vec<Channel> {
        // 模拟通道号个数
        let byChanNum = self.0.byChanNum;
        // 模拟通道号起始号
        let byStartChan = self.0.byStartChan;
        // IP通道（或者数字通道）支持的最大IP通道数
        let maxIPChan = self.0.byIPChanNum as u16 + (self.0.byHighDChanNum as u16 * 256);
        // IP通道（或者数字通道）起始通道号
        let byStartDChan = self.0.byStartDChan as u16;
        let mut channels = Vec::new();

        let mut index = 0;
        let end = byStartChan + byChanNum;
        for num in byStartChan..end {
            channels.push(Channel::Logic(ChannelInfo {
                index,
                chan_num: num as u16,
            }));

            index += 1;
        }

        index = 0;
        let end = byStartDChan + maxIPChan;
        for num in byStartDChan..end {
            channels.push(Channel::IP(ChannelInfo {
                index,
                chan_num: num as u16,
            }));

            index += 1;
        }
        channels
    }
}
