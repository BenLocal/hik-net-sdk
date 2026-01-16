use std::{
    mem,
    os::raw::c_char,
    sync::atomic::{AtomicBool, Ordering},
};

use chrono::{DateTime, Datelike as _, Local, Timelike as _};

use crate::{
    DWORD, LONG, LPNET_DVR_DEVICEINFO_V30, NET_DVR_CaptureJPEGPicture, NET_DVR_DEVICEINFO_V30,
    NET_DVR_GET_IPPARACFG_V40, NET_DVR_GetDVRConfig, NET_DVR_GetDownloadPos,
    NET_DVR_GetFileByTime_V40, NET_DVR_IPPARACFG_V40, NET_DVR_JPEGPARA, NET_DVR_Login_V30,
    NET_DVR_Logout_V30, NET_DVR_PLAYCOND, NET_DVR_PLAYSTART, NET_DVR_PlayBackControl_V40,
    NET_DVR_StopGetFile, NET_DVR_TIME, as_c_string, common::get_last_error_code,
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

    pub fn get_channels(&self) -> anyhow::Result<Vec<Channel>> {
        let channel_config = self.get_ip_channel_config()?;
        let mut channels = match self.device_info.as_ref() {
            Some(device_info) => device_info.get_channels(),
            None => return Err(anyhow::anyhow!("Device info not found")),
        };

        for channel in channels.iter_mut() {
            match channel {
                Channel::Logic(channel) => {
                    channel.enable = channel_config.byAnalogChanEnable[channel.index as usize] == 1;
                }
                Channel::IP(channel) => {
                    let ip_dev_info = channel_config.struIPDevInfo[channel.index as usize];
                    let stream_mode = channel_config.struStreamMode[channel.index as usize];
                    channel.enable = ip_dev_info.byEnable == 1;

                    // Safely convert i8 arrays to u8 slices for valid UTF-8 conversion
                    let ip_address: String = {
                        let raw = &ip_dev_info.struIP.sIpV4;
                        let u8_slice = unsafe {
                            std::slice::from_raw_parts(raw.as_ptr() as *const u8, raw.len())
                        };
                        String::from_utf8_lossy(u8_slice)
                            .trim_end_matches('\0')
                            .to_string()
                    };

                    channel.ipv4_address = Some(ip_address);

                    let ipv6_address: String = {
                        let raw = &ip_dev_info.struIP.byIPv6;
                        let u8_slice = unsafe {
                            std::slice::from_raw_parts(raw.as_ptr() as *const u8, raw.len())
                        };
                        String::from_utf8_lossy(u8_slice)
                            .trim_end_matches('\0')
                            .to_string()
                    };
                    channel.ipv6_address = Some(ipv6_address);

                    let stream_type = stream_mode.byGetStreamType;
                    channel.get_stream_type = Some(stream_type);
                    if stream_type == 0 {
                        let stream = unsafe { stream_mode.uGetStream.struChanInfo };
                        channel.stream_channel = Some(stream.byChannel);
                    }
                }
            }
        }

        Ok(channels)
    }

    fn get_ip_channel_config(&self) -> anyhow::Result<NET_DVR_IPPARACFG_V40> {
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

        // true is success, false is failed
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

        if res != 1 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Get IP channel config failed: error code {}, dwReturned: {}",
                error_code,
                dw_returned
            ));
        }

        Ok(ip_access_cfg_v40)
    }

    pub fn capture_jpeg_picture(&self, channel: u16, file: &str) -> anyhow::Result<()> {
        let lu = self
            .login_hanlder
            .ok_or(anyhow::anyhow!("Login hanlder not found"))?;

        let mut params = NET_DVR_JPEGPARA::default();
        let file = as_c_string!(file);
        let res = unsafe {
            NET_DVR_CaptureJPEGPicture(
                lu,
                channel as i32,
                &mut params as *mut _,
                file.as_ptr() as *mut c_char,
            )
        };
        if res != 1 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Capture JPEG picture failed: error code {}",
                error_code
            ));
        }
        Ok(())
    }

    pub fn get_file_by_time(
        &self,
        file: &str,
        channel: u16,
        start_time: DateTime<Local>,
        end_time: DateTime<Local>,
    ) -> anyhow::Result<HikDownload> {
        let lu = self
            .login_hanlder
            .ok_or(anyhow::anyhow!("Login hanlder not found"))?;

        let file = as_c_string!(file);
        let mut play_cond = NET_DVR_PLAYCOND::default();
        play_cond.dwChannel = channel as DWORD;
        play_cond.struStartTime = NET_DVR_TIME {
            dwYear: start_time.year() as DWORD,
            dwMonth: start_time.month() as DWORD,
            dwDay: start_time.day() as DWORD,
            dwHour: start_time.hour() as DWORD,
            dwMinute: start_time.minute() as DWORD,
            dwSecond: start_time.second() as DWORD,
        };
        play_cond.struStopTime = NET_DVR_TIME {
            dwYear: end_time.year() as DWORD,
            dwMonth: end_time.month() as DWORD,
            dwDay: end_time.day() as DWORD,
            dwHour: end_time.hour() as DWORD,
            dwMinute: end_time.minute() as DWORD,
            dwSecond: end_time.second() as DWORD,
        };
        let handle = unsafe {
            NET_DVR_GetFileByTime_V40(lu, file.as_ptr() as *mut c_char, &mut play_cond as *mut _)
        };

        if handle < 0 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Get file by time failed: error code {}",
                error_code
            ));
        }

        Ok(HikDownload::new(handle))
    }
}

pub struct HikDownload {
    handle: i32,
    is_start: AtomicBool,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl HikDownload {
    pub fn new(handle: i32) -> Self {
        Self {
            handle,
            is_start: AtomicBool::new(false),
            thread: None,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.is_start.load(Ordering::Relaxed) {
            return Ok(());
        }
        let res = unsafe {
            NET_DVR_PlayBackControl_V40(
                self.handle as LONG,
                NET_DVR_PLAYSTART,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if res != 1 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Start download failed: error code {}",
                error_code
            ));
        }
        self.is_start.store(true, Ordering::Relaxed);

        Ok(())
    }

    pub fn get_progress(&self) -> anyhow::Result<i32> {
        if !self.is_start.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Download not started"));
        }

        let pos = unsafe { NET_DVR_GetDownloadPos(self.handle as LONG) };
        if pos < 0 || pos > 100 {
            if pos == -1 {
                let error_code = get_last_error_code();
                return Err(anyhow::anyhow!(
                    "Get download progress failed: error code {}",
                    error_code
                ));
            } else if pos == 200 {
                return Err(anyhow::anyhow!("Get download network error"));
            }

            return Err(anyhow::anyhow!("Get download progress failed"));
        }
        Ok(pos)
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.is_start.store(false, Ordering::Relaxed);
        let res = unsafe { NET_DVR_StopGetFile(self.handle as LONG) };
        if res != 1 {
            let error_code = get_last_error_code();
            return Err(anyhow::anyhow!(
                "Stop download failed: error code {}",
                error_code
            ));
        }
        Ok(())
    }
}

impl Drop for HikDownload {
    fn drop(&mut self) {
        let _ = self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug)]
pub enum Channel {
    Logic(ChannelInfo),
    IP(ChannelInfo),
}

#[derive(Debug, Default)]
pub struct ChannelInfo {
    index: u16,
    chan_num: u16,
    enable: bool,
    get_stream_type: Option<u8>,
    stream_channel: Option<u8>,
    ipv4_address: Option<String>,
    ipv6_address: Option<String>,
}

impl ChannelInfo {
    pub fn new(index: u16, chan_num: u16) -> Self {
        Self {
            index,
            chan_num,
            enable: false,
            ..Default::default()
        }
    }

    pub fn get_chan_num(&self) -> u16 {
        self.chan_num
    }
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
            channels.push(Channel::Logic(ChannelInfo::new(index, num as u16)));
            index += 1;
        }

        index = 0;
        let end = byStartDChan + maxIPChan;
        for num in byStartDChan..end {
            channels.push(Channel::IP(ChannelInfo::new(index, num as u16)));
            index += 1;
        }
        channels
    }
}
