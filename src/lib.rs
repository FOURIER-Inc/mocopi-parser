use nom::bytes::complete::take;
use nom::error::Error;
use nom::number::complete::le_u32;

pub type BoneId = u16;
pub type TransVal = f32;

#[derive(Debug, PartialEq)]
pub struct SkeletonPacket {
    pub head: Head,
    pub info: Info,
    pub skeleton: Skeleton,
}

#[derive(Debug, PartialEq)]
pub struct Head {
    pub format: String,
    pub ver: u8,
}

#[derive(Debug, PartialEq)]
pub struct Info {
    pub addr: u64,
    pub port: u16,
}

#[derive(Debug, PartialEq)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

#[derive(Debug, PartialEq)]
pub struct Bone {
    pub id: BoneId,
    pub parent: BoneId,
    pub trans: Transform,
}

#[derive(Debug, PartialEq)]
pub struct FramePacket {
    pub head: Head,
    pub info: Info,
    pub frame: Frame,
}

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub num: u32,
    pub time: u32,
    pub bones: Vec<BoneTrans>,
}

#[derive(Debug, PartialEq)]
pub struct BoneTrans {
    pub id: BoneId,
    pub trans: Transform,
}

#[derive(Debug, PartialEq)]
pub struct Transform {
    pub rot: Rotation,
    pub pos: Position,
}

#[derive(Debug, PartialEq)]
pub struct Rotation {
    pub x: TransVal,
    pub y: TransVal,
    pub z: TransVal,
    pub w: TransVal,
}

#[derive(Debug, PartialEq)]
pub struct Position {
    pub x: TransVal,
    pub y: TransVal,
    pub z: TransVal,
}

#[derive(Debug, PartialEq)]
struct Data<'a> {
    pub len: u32,
    pub name: String,
    pub data: &'a [u8],
    pub rem: &'a [u8],
}

fn parse_value(data: &[u8]) -> Data {
    // lengthの長さは4bytesで固定
    let (data, length) = le_u32::<_, Error<_>>(data).unwrap() as (&[u8], u32);

    // nameは4bytesの文字列
    let (data, name) = take::<_, _, Error<_>>(4usize)(data).unwrap();
    let name_str = String::from_utf8(name.to_vec()).unwrap();

    // valueの長さはlengthの値による
    let (rem, data) = take::<_, _, Error<_>>(length)(data).unwrap();

    return Data {
        len: length,
        name: name_str,
        data,
        rem,
    };
}

fn parse_head(data: &[u8]) -> (u32, Head) {
    let data = parse_value(data);
    let len = data.len;

    // ftyp
    let data = parse_value(data.data);
    let format = String::from_utf8(data.data.to_vec()).unwrap();

    // vrsn
    let data = parse_value(data.rem);
    let ver = data.data[0];

    (len, Head { format, ver })
}

fn parse_info(data: &[u8]) -> (u32, Info) {
    let data = parse_value(data);
    let len = data.len;

    // ipad
    let data = parse_value(data.data);
    let addr = u64::from_le_bytes(data.data.try_into().unwrap());

    // rcvp
    let data = parse_value(data.rem);
    let port = u16::from_le_bytes(data.data.try_into().unwrap());

    (len, Info { addr, port })
}

fn parse_skeleton(data: &[u8]) -> (u32, Skeleton) {
    // skdf
    let data = parse_value(data);
    let len = data.len;

    // bons
    let (_, bones) = parse_bones(data.data);

    (len, Skeleton { bones: *bones })
}

fn parse_frame(data: &[u8]) -> (u32, Frame) {
    // fram
    let data = parse_value(data);
    let len = data.len;

    // fnum
    let data = parse_value(data.data);
    let num = u32::from_le_bytes(data.data.try_into().unwrap());

    // time
    let data = parse_value(data.rem);
    let time = u32::from_le_bytes(data.data.try_into().unwrap());

    // btrs
    let (_, bones) = parse_bone_trans(data.rem);

    (len, Frame { num, time, bones: *bones })
}

fn parse_bone_trans(data: &[u8]) -> (u32, Box<Vec<BoneTrans>>) {
    // btrs
    let btrs_data = parse_value(data);
    let btrs_len = btrs_data.len;

    // btrsの下にあるbtdtをparseしていく
    let mut bones: Vec<BoneTrans> = Vec::new();
    let mut read_bytes: u32 = 0;
    loop {
        let part = &btrs_data.data[(read_bytes as usize)..];

        // btdt
        let data = parse_value(part);
        let len = data.len;

        // bnid
        let data = parse_value(data.data);
        let id = u16::from_le_bytes(data.data.try_into().unwrap());

        // tran
        let (_, trans) = parse_trans(data.rem);

        bones.push(BoneTrans { id, trans });

        read_bytes += len + 8;
        if read_bytes == btrs_len {
            break;
        }
    }

    (btrs_len, Box::new(bones))
}

fn parse_bones(data: &[u8]) -> (u32, Box<Vec<Bone>>) {
    // bons
    let bons_data = parse_value(data);
    let bons_len = bons_data.len;

    // bonsの下にあるbndtをparseしていく
    let mut bones: Vec<Bone> = Vec::new();
    let mut read_bytes: u32 = 0;
    loop {
        let part = &bons_data.data[(read_bytes as usize)..];

        // bndt
        let data = parse_value(part);
        let len = data.len;

        // bnid
        let data = parse_value(data.data);
        let id = u16::from_le_bytes(data.data.try_into().unwrap());

        // pbid
        let data = parse_value(data.rem);
        let parent = u16::from_le_bytes(data.data.try_into().unwrap());

        // tran
        let (_, trans) = parse_trans(part);

        bones.push(Bone { id, parent, trans });

        read_bytes += len + 8;
        if read_bytes == bons_len {
            break;
        }
    }

    (bons_len, Box::new(bones))
}

fn parse_trans(data: &[u8]) -> (u32, Transform) {
    // tran
    let data = parse_value(data);
    let len = data.len;

    // 28bytesのデータを4bytesごとに取り出す
    let mut values = [0.0; 7];
    for i in 0..6usize {
        let v = data.data[i * 4..(i * 4 + 4)].to_vec();
        values[i] = f32::from_le_bytes(v.try_into().unwrap());
    }

    (len, Transform {
        rot: Rotation { x: values[0], y: values[1], z: values[2], w: values[3] },
        pos: Position { x: values[4], y: values[5], z: values[6] },
    })
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
