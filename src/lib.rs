use nom::bytes::complete::take;
use nom::error::Error;
use nom::number::complete::le_u32;
use serde::{Deserialize, Serialize};
use std::error;

pub type BoneId = u16;
pub type TransVal = f32;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SkeletonPacket {
    pub head: Head,
    pub info: Info,
    pub skeleton: Skeleton,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Head {
    pub format: String,
    pub ver: u8,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Info {
    pub addr: u64,
    pub port: u16,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Bone {
    pub id: BoneId,
    pub parent: BoneId,
    pub trans: Transform,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FramePacket {
    pub head: Head,
    pub info: Info,
    pub frame: Frame,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub num: u32,
    pub time: u32,
    pub bones: Vec<BoneTrans>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct BoneTrans {
    pub id: BoneId,
    pub trans: Transform,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub rot: Rotation,
    pub pos: Position,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Rotation {
    pub x: TransVal,
    pub y: TransVal,
    pub z: TransVal,
    pub w: TransVal,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: TransVal,
    pub y: TransVal,
    pub z: TransVal,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Data<'a> {
    pub len: u32,
    pub name: String,
    pub data: &'a [u8],
    pub rem: &'a [u8],
}

pub enum SkeletonOrFrame {
    Skeleton(SkeletonPacket),
    Frame(FramePacket),
}

/// Parse the values.
fn parse_value(data: &[u8]) -> Result<Data, Box<dyn error::Error + '_>> {
    // lengthの長さは4bytesで固定
    let (data, length) = le_u32::<_, Error<_>>(data)? as (&[u8], u32);

    // nameは4bytesの文字列
    let (data, name) = take::<_, _, Error<_>>(4usize)(data)?;
    let name_str = String::from_utf8(name.to_vec())?;

    // valueの長さはlengthの値による
    let (rem, data) = take::<_, _, Error<_>>(length)(data)?;

    Ok(Data {
        len: length,
        name: name_str,
        data,
        rem,
    })
}

fn parse_head(data: &[u8]) -> Result<(u32, Head), Box<dyn error::Error + '_>> {
    let data = parse_value(data)?;
    let len = data.len;

    // ftyp
    let data = parse_value(data.data)?;
    let format = String::from_utf8(data.data.to_vec())?;

    // vrsn
    let data = parse_value(data.rem)?;
    let ver = data.data[0];

    Ok((len, Head { format, ver }))
}

fn parse_info(data: &[u8]) -> Result<(u32, Info), Box<dyn error::Error + '_>> {
    let data = parse_value(data)?;
    let len = data.len;

    // ipad
    let data = parse_value(data.data)?;
    let addr = u64::from_le_bytes(data.data.try_into()?);

    // rcvp
    let data = parse_value(data.rem)?;
    let port = u16::from_le_bytes(data.data.try_into()?);

    Ok((len, Info { addr, port }))
}

fn parse_skeleton(data: &[u8]) -> Result<(u32, Skeleton), Box<dyn error::Error + '_>> {
    // skdf
    let data = parse_value(data)?;
    let len = data.len;

    // bons
    let (_, bones) = parse_bones(data.data)?;

    Ok((len, Skeleton { bones: *bones }))
}

fn parse_frame(data: &[u8]) -> Result<(u32, Frame), Box<dyn error::Error + '_>> {
    // fram
    let data = parse_value(data)?;
    let len = data.len;

    // fnum
    let data = parse_value(data.data)?;
    let num = u32::from_le_bytes(data.data.try_into()?);

    // time
    let data = parse_value(data.rem)?;
    let time = u32::from_le_bytes(data.data.try_into()?);

    // btrs
    let (_, bones) = parse_bone_trans(data.rem)?;

    Ok((
        len,
        Frame {
            num,
            time,
            bones: *bones,
        },
    ))
}

fn parse_bone_trans(data: &[u8]) -> Result<(u32, Box<Vec<BoneTrans>>), Box<dyn error::Error + '_>> {
    // btrs
    let btrs_data = parse_value(data)?;
    let btrs_len = btrs_data.len;

    // btrsの下にあるbtdtをparseしていく
    let mut bones: Vec<BoneTrans> = Vec::new();
    let mut read_bytes: u32 = 0;
    loop {
        let part = &btrs_data.data[(read_bytes as usize)..];

        // btdt
        let data = parse_value(part)?;
        let len = data.len;

        // bnid
        let data = parse_value(data.data)?;
        let id = u16::from_le_bytes(data.data.try_into()?);

        // tran
        let (_, trans) = parse_trans(data.rem)?;

        bones.push(BoneTrans { id, trans });

        read_bytes += len + 8;
        if read_bytes == btrs_len {
            break;
        }
    }

    Ok((btrs_len, Box::new(bones)))
}

fn parse_bones(data: &[u8]) -> Result<(u32, Box<Vec<Bone>>), Box<dyn error::Error + '_>> {
    // bons
    let bons_data = parse_value(data)?;
    let bons_len = bons_data.len;

    // bonsの下にあるbndtをparseしていく
    let mut bones: Vec<Bone> = Vec::new();
    let mut read_bytes: u32 = 0;
    loop {
        let part = &bons_data.data[(read_bytes as usize)..];

        // bndt
        let data = parse_value(part)?;
        let len = data.len;

        // bnid
        let data = parse_value(data.data)?;
        let id = u16::from_le_bytes(data.data.try_into()?);

        // pbid
        let data = parse_value(data.rem)?;
        let parent = u16::from_le_bytes(data.data.try_into()?);

        // tran
        let (_, trans) = parse_trans(part)?;

        bones.push(Bone { id, parent, trans });

        read_bytes += len + 8;
        if read_bytes == bons_len {
            break;
        }
    }

    Ok((bons_len, Box::new(bones)))
}

fn parse_trans(data: &[u8]) -> Result<(u32, Transform), Box<dyn error::Error + '_>> {
    // tran
    let data = parse_value(data)?;

    // 28bytesのデータを4bytesごとに取り出す
    let mut values = [0.0; 7];
    for (i, v) in values.iter_mut().enumerate() {
        let b = &data.data[i * 4..(i * 4 + 4)];
        *v = f32::from_le_bytes(b.try_into()?);
    }

    Ok((
        data.len,
        Transform {
            rot: Rotation {
                x: values[0],
                y: values[1],
                z: values[2],
                w: values[3],
            },
            pos: Position {
                x: values[4],
                y: values[5],
                z: values[6],
            },
        },
    ))
}

/// Parse the streamed data from mocopi.
///
/// # Examples
///
/// ```
/// use std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("192.168.10.1:12351").unwrap();
/// let mut buf = [0; 1024];
///
/// loop {
///     socket.recv_from(&mut buf).unwrap();
///     let packet = mocopi_parser::parse(&mut buf).unwrap();
///
///     match packet {
///         mocopi_parser::SkeletonOrFrame::Skeleton(skeleton) => { dbg!(skeleton); },
///         mocopi_parser::SkeletonOrFrame::Frame(frame) => { dbg!(frame); },
///     }
/// }
/// ```
pub fn parse(data: &mut [u8]) -> Result<SkeletonOrFrame, Box<dyn error::Error + '_>> {
    let (len, head) = parse_head(data)?;
    let mut remain = &data[((len + 8) as usize)..];

    let (len, info) = parse_info(remain)?;
    remain = &remain[((len + 8) as usize)..];

    let name = parse_value(data)?.name;

    if name == "skdf" {
        let (_, skeleton) = parse_skeleton(remain)?;
        Ok(SkeletonOrFrame::Skeleton(SkeletonPacket {
            head,
            info,
            skeleton,
        }))
    } else {
        let (_, frame) = parse_frame(remain)?;
        Ok(SkeletonOrFrame::Frame(FramePacket { head, info, frame }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        let raw = [
            0x04, 0x00, 0x00, 0x00,
            0x62, 0x6e, 0x64, 0x74,
            0x02, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00
        ];

        let data = parse_value(&raw).unwrap();

        assert_eq!(data.len, 4);
        assert_eq!(data.name, "bndt");
        assert_eq!(data.data, [0x02, 0x00, 0x00, 0x00]);
        assert_eq!(data.rem, [0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_parse_trans() {
        let raw = [
            0x1c, 0x00, 0x00, 0x00,

            0x74, 0x72, 0x61, 0x6e,

            0x00, 0x00, 0x9c, 0xa2,
            0x00, 0xc0, 0xfe, 0xa4,
            0x00, 0x00, 0xd0, 0xa3,
            0x00, 0x00, 0x80, 0x3f,

            0x17, 0x56, 0x03, 0xbc,
            0x7c, 0x48, 0xd0, 0xbd,
            0x0c, 0xa8, 0x03, 0x3e,
        ];

        let (len, data) = parse_trans(&raw).unwrap();

        assert_eq!(len, 28);

        assert_eq!(data.rot.x, -4.22838847e-18);
        assert_eq!(data.rot.y, -1.104802e-16);
        assert_eq!(data.rot.z, -2.25514052e-17);
        assert_eq!(data.rot.w, 1.0);

        assert_eq!(data.pos.x, -0.008016131);
        assert_eq!(data.pos.y, -0.101700753);
        assert_eq!(data.pos.z, 0.128570735);
    }
}
