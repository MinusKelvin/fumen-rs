
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Fumen {
    pub pages: Vec<Page>,
    pub guideline: bool
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Page {
    pub piece: Option<Piece>,
    /// y-up
    pub field: [[CellColor; 10]; 23],
    pub garbage_row: [CellColor; 10],
    pub rise: bool,
    pub mirror: bool,
    pub lock: bool,
    pub comment: Option<String>
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PieceType {
    I = 1,
    L = 2,
    O = 3,
    Z = 4,
    T = 5,
    J = 6,
    S = 7
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum RotationState {
    South = 0,
    East = 1,
    North = 2,
    West = 3
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Piece {
    pub kind: PieceType,
    /// SRS true rotation
    pub rotation: RotationState,
    pub x: u32,
    /// y-up
    pub y: u32
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellColor {
    Empty = 0,
    I = 1,
    L = 2,
    O = 3,
    Z = 4,
    T = 5,
    J = 6,
    S = 7,
    Grey = 8
}

const BASE64_CHARS: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J',
    b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T',
    b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd',
    b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n',
    b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x',
    b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b'+', b'/'
];

impl Fumen {
    pub fn encode(&self) -> String {
        // we need a vec and not a string here since we need to go back and patch in the
        // length of empty field sequences... and i don't want to do 2-pass encoding
        let mut data = b"v115@".to_vec();
        let mut prev_field = [[CellColor::Empty; 10]; 24];
        let mut empty_field = None;
        let mut first = true;
        for page in &self.pages {
            // encode field
            let deltas = fumen_field_delta(prev_field, page.fumen_field());
            if deltas == [[8; 10]; 24] {
                // do special-case unchanged field stuff
                if let Some((ref mut index, ref mut count)) = empty_field {
                    // count empty fields
                    *count += 1;
                    if *count == 63 {
                        data[*index] = BASE64_CHARS[*count];
                        empty_field = None;
                    }
                } else {
                    // new empty field encoding
                    data.push(b'v');
                    data.push(b'h');
                    empty_field = Some((data.len(), 0));
                    data.push(0);
                }
            } else {
                // finalize the empty field sequence
                if let Some((index, count)) = empty_field {
                    data[index] = BASE64_CHARS[count];
                    empty_field = None;
                }
                // do run-length encoding of deltas
                let mut prev = deltas[0][0];
                let mut count = 0;
                for y in 0..24 {
                    for x in 0..10 {
                        if deltas[y][x] == prev {
                            count += 1;
                        } else {
                            let num = prev * 240 + count - 1;
                            data.push(BASE64_CHARS[num & 0x3F]);
                            data.push(BASE64_CHARS[num >> 6 & 0x3F]);
                            prev = deltas[y][x];
                            count = 1;
                        }
                    }
                }
                let num = prev * 240 + count - 1;
                data.push(BASE64_CHARS[num & 0x3F]);
                data.push(BASE64_CHARS[num >> 6 & 0x3F]);
            }

            let page_flags = page.fumen_number() as usize + if first {
                first = false;
                self.guideline as usize * 240 * 128
            } else { 0 };
            data.push(BASE64_CHARS[page_flags & 0x3F]);
            data.push(BASE64_CHARS[page_flags >> 6 & 0x3F]);
            data.push(BASE64_CHARS[page_flags >> 12 & 0x3F]);

            if let Some(ref comment) = page.comment {
                let mut encoded = js_escape(comment);
                encoded.truncate(4095);
                data.push(BASE64_CHARS[encoded.len() & 0x3F]);
                data.push(BASE64_CHARS[encoded.len() >> 6 & 0x3F]);

                for c in encoded.chunks(4) {
                    let mut v = 0;
                    for &c in c.iter().rev() {
                        v *= 96;
                        v += c as usize - 0x20;
                    }
                    for _ in 0..5 {
                        data.push(BASE64_CHARS[v & 0x3F]);
                        v >>= 6;
                    }
                }
            }

            // this handles piece locking, line clear, mirror, and rise rules
            prev_field = page.next_page().fumen_field();
        }

        // finalize the empty field sequence
        if let Some((index, count)) = empty_field {
            data[index] = BASE64_CHARS[count];
        }

        String::from_utf8(data).unwrap()
    }

    pub fn decode(data: &str) -> Option<Fumen> {
        unimplemented!()
    }

    pub fn add_page(&mut self) -> &mut Page {
        self.pages.push(self.pages.last().unwrap().next_page());
        self.pages.last_mut().unwrap()
    }
}

fn fumen_field_delta(
    from: [[CellColor; 10]; 24], to: [[CellColor; 10]; 24]
) -> [[usize; 10]; 24] {
    let mut deltas = [[0; 10]; 24];
    for y in 0..24 {
        for x in 0..10 {
            deltas[y][x] = 8 + to[y][x] as usize - from[y][x] as usize
        }
    }
    deltas
}

impl Page {
    fn fumen_number(&self) -> u32 {
        self.piece.map(|p| p.fumen_number()).unwrap_or(0) + 240 * 32 * (
            self.rise as u32 +
            2 * self.mirror as u32 +
            8 * self.comment.is_some() as u32 +
            16 * !self.lock as u32
        )
    }

    fn fumen_field(&self) -> [[CellColor; 10]; 24] {
        let mut field = [[CellColor::Empty; 10]; 24];
        for y in 0..23 {
            field[22-y] = self.field[y];
        }
        field[23] = self.garbage_row;
        field
    }

    pub fn next_page(&self) -> Page {
        let mut field = self.field;

        // do piece placement
        if let Some(piece) = self.piece {
            if self.lock {
                for &(x, y) in &piece.cells() {
                    field[y as usize][x as usize] = piece.kind.into();
                }
            }
        }

        // do line clear rule
        let mut y = 0;
        for i in 0..23 {
            let mut cleared = true;
            for x in 0..10 {
                if field[i][x] == CellColor::Empty {
                    cleared = false;
                }
            }
            if !cleared {
                field[y] = field[i];
                y += 1;
            }
        }
        for i in y..23 {
            field[i] = [CellColor::Empty; 10];
        }

        // do "rise" rule
        if self.rise {
            for i in (1..23).rev() {
                field[i] = field[i-1];
            }
            field[0] = self.garbage_row;
        }

        // do "mirror" rule
        if self.mirror {
            for row in &mut field {
                row.reverse();
            }
        }

        Page {
            piece: None,
            comment: None,
            rise: false,
            mirror: false,
            lock: self.lock,
            field,
            garbage_row: if self.rise {
                [CellColor::Empty; 10]
            } else {
                self.garbage_row
            }
        }
    }
}

impl Piece {
    fn fumen_number(&self) -> u32 {
        self.kind as u32 +
            8 * self.rotation as u32 +
            32 * self.fumen_pos()
    }

    fn fumen_pos(&self) -> u32 {
        // Convert true SRS piece centers to fumen's system
        let x = match (self.kind, self.rotation) {
            (PieceType::S, RotationState::East) => self.x + 1,
            (PieceType::Z, RotationState::West) => self.x - 1,
            (PieceType::O, RotationState::West) => self.x - 1,
            (PieceType::O, RotationState::South) => self.x - 1,
            (PieceType::I, RotationState::South) => self.x - 1,
            _ => self.x
        };
        let y = match (self.kind, self.rotation) {
            (PieceType::S, RotationState::North) => self.y + 1,
            (PieceType::Z, RotationState::North) => self.y + 1,
            (PieceType::O, RotationState::North) => self.y + 1,
            (PieceType::O, RotationState::West) => self.y + 1,
            (PieceType::I, RotationState::West) => self.y + 1,
            _ => self.y
        };

        x + (22 - y) * 10
    }

    fn cells(&self) -> [(i32, i32); 4] {
        let mut cells = match self.kind {
            PieceType::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
            PieceType::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            PieceType::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
            PieceType::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
            PieceType::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
            PieceType::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            PieceType::Z => [(1, 0), (0, 0), (0, 1), (-1, 1)]
        };

        for (x, y) in &mut cells {
            match self.rotation {
                RotationState::North => {}
                RotationState::East => {
                    std::mem::swap(x, y);
                    *y = -*y;
                }
                RotationState::South => {
                    *x = -*x;
                    *y = -*y;
                }
                RotationState::West => {
                    std::mem::swap(x, y);
                    *x = -*x;
                }
            }

            *x += self.x as i32;
            *y += self.y as i32;
        }

        cells
    }
}

impl Default for Fumen {
    fn default() -> Self {
        Fumen {
            pages: vec![Page::default()],
            guideline: true
        }
    }
}

impl Default for Page {
    fn default() -> Self {
        Page {
            piece: None,
            field: [[CellColor::Empty; 10]; 23],
            garbage_row: [CellColor::Empty; 10],
            rise: false,
            mirror: false,
            lock: true,
            comment: None
        }
    }
}

impl From<PieceType> for CellColor {
    fn from(v: PieceType) -> CellColor {
        match v {
            PieceType::I => CellColor::I,
            PieceType::L => CellColor::L,
            PieceType::O => CellColor::O,
            PieceType::Z => CellColor::Z,
            PieceType::T => CellColor::T,
            PieceType::J => CellColor::J,
            PieceType::S => CellColor::S,
        }
    }
}

fn js_escape(s: &str) -> Vec<u8> {
    const HEX_DIGITS: [u8; 16] = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
        b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F'
    ];

    let mut result = Vec::new();
    for c in s.chars() {
        match c {
            'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' |
            '@' | '*' | '_' | '+' | '-' | '.' | '/' => result.push(c as u8),
            '\u{0}' ..= '\u{FF}' => {
                result.push(b'%');
                result.push(HEX_DIGITS[(c as usize) >> 4 & 0xF]);
                result.push(HEX_DIGITS[(c as usize) >> 0 & 0xF]);
            }
            _ => {
                let mut buf = [0; 2];
                for &mut c in c.encode_utf16(&mut buf) {
                    result.extend_from_slice(b"%u");
                    result.push(HEX_DIGITS[(c as usize) >> 12 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 8 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 4 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 0 & 0xF]);
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn empty() {
        let fumen = Fumen::default();
        assert_eq!(fumen.encode(), "v115@vhAAgH");
        // assert_eq!(Fumen::decode("v115@vhAAgH"), Some(fumen));
    }

    #[test]
    fn one_page_lock_piece() {
        let mut fumen = Fumen::default();
        fumen.pages[0].piece = Some(Piece {
            kind: PieceType::T,
            rotation: RotationState::North,
            x: 2,
            y: 0
        });
        assert_eq!(fumen.encode(), "v115@vhAVPJ");
        // assert_eq!(Fumen::decode("v115@vhAVPJ"), Some(fumen));
    }

    #[test]
    fn lock_piece() {
        let mut fumen = Fumen::default();
        fumen.pages[0].piece = Some(Piece {
            kind: PieceType::T,
            rotation: RotationState::North,
            x: 2,
            y: 0
        });
        fumen.pages.push(Page::default());
        assert_eq!(fumen.encode(), "v115@vhAVPJThQLHeSLPeAAA");
        // assert_eq!(Fumen::decode("v115@vhAVPJThQLHeSLPeAAA"), Some(fumen));
    }

    #[test]
    fn o_piece_wobble() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[2][3] = CellColor::Grey;
        fumen.pages[0].field[5][3] = CellColor::Grey;
        fumen.pages[0].field[8][3] = CellColor::Grey;
        fumen.pages[0].piece = Some(Piece {
            kind: PieceType::O,
            rotation: RotationState::North,
            x: 3, y: 0
        });
        fumen.add_page().piece = Some(Piece {
            kind: PieceType::O,
            rotation: RotationState::West,
            x: 4, y: 3
        });
        fumen.add_page().piece = Some(Piece {
            kind: PieceType::O,
            rotation: RotationState::South,
            x: 4, y: 7
        });
        fumen.add_page().piece = Some(Piece {
            kind: PieceType::O,
            rotation: RotationState::East,
            x: 3, y: 10
        });
        fumen.pages.push(Page::default());
        assert_eq!(
            fumen.encode(),
            "v115@OgA8ceA8ceA8jezKJvhC7bBjMBr9A6fxSHexSHeAAIexSHexSHeAAIexSHexSHeAAIexSHexSOeAAA"
        );
        // assert_eq!(Fumen::decode(
        //     "v115@OgA8ceA8ceA8jezKJvhC7bBjMBr9A6fxSHexSHeAAIexSHexSHeAAIexSHexSHeAAIexSHexSOeAAA"
        // ), Some(fumen));
    }

    #[test]
    fn fumen_field() {
        let mut page = Page::default();
        page.field[0] = [CellColor::Grey; 10];
        page.garbage_row[0] = CellColor::Grey;
        let mut fumen_field = [[CellColor::Empty; 10]; 24];
        fumen_field[22] = [CellColor::Grey; 10];
        fumen_field[23][0] = CellColor::Grey;
        assert_eq!(page.fumen_field(), fumen_field);
    }

    #[test]
    fn fumen_field_deltas() {
        let mut page = Page::default();
        let empty = page.fumen_field();
        page.field[0] = [CellColor::Grey; 10];
        page.garbage_row[0] = CellColor::Grey;
        let mut deltas = [[8; 10]; 24];
        deltas[22] = [16; 10];
        deltas[23][0] = 16;
        assert_eq!(fumen_field_delta(empty, page.fumen_field()), deltas);
    }

    #[test]
    fn simple_field() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[22][0] = CellColor::Grey;
        assert_eq!(fumen.encode(), "v115@A8uhAgH");
        // assert_eq!(Fumen::decode("v115@A8uhAgH"), Some(fumen));
    }

    #[test]
    fn arbitrary_field() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[0] = [CellColor::Grey; 10];
        fumen.pages[0].field[0][4] = CellColor::Empty;
        fumen.pages[0].field[0][7] = CellColor::T;
        fumen.pages[0].field[1] = [CellColor::S; 10];
        fumen.pages[0].field[1][1] = CellColor::Empty;
        fumen.pages[0].field[1][9] = CellColor::L;
        fumen.pages[0].field[2] = [CellColor::Z; 10];
        fumen.pages[0].field[2][6] = CellColor::Empty;
        fumen.pages[0].field[2][2] = CellColor::O;
        fumen.pages[0].field[3] = [CellColor::I; 10];
        fumen.pages[0].field[3][2] = CellColor::Empty;
        fumen.pages[0].field[3][6] = CellColor::J;
        assert_eq!(fumen.encode(), "v115@9gxhAeyhg0yhBtQpCtAeCtQ4AeW4glD8AeB8wwB8JeAgH");
        // assert_eq!(
        //     Fumen::decode("v115@9gxhAeyhg0yhBtQpCtAeCtQ4AeW4glD8AeB8wwB8JeAgH"),
        //     Some(fumen)
        // );
    }

    #[test]
    fn line_clear() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[0] = [CellColor::Grey; 10];
        fumen.add_page();
        assert_eq!(fumen.encode(), "v115@bhJ8JeAgHvhAAAA");
        // assert_eq!(Fumen::decode("v115@bhJ8JeAgHvhAAAA"), Some(fumen));
    }

    #[test]
    fn rise() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[0][1] = CellColor::I;
        fumen.pages[0].garbage_row[4] = CellColor::Grey;
        fumen.pages[0].rise = true;
        fumen.add_page();
        fumen.pages.push(Page::default());
        assert_eq!(fumen.encode(), "v115@chwhLeA8EeAYJvhAAAAShQaLeAAOeAAA");
        // assert_eq!(Fumen::decode("v115@chwhLeA8EeAYJvhAAAAShQaLeAAOeAAA"), Some(fumen));
    }

    #[test]
    fn mirror() {
        let mut fumen = Fumen::default();
        fumen.pages[0].field[0] = [
            CellColor::I, CellColor::L, CellColor::O, CellColor::Z, CellColor::T,
            CellColor::J, CellColor::S, CellColor::Grey, CellColor::Empty, CellColor::Empty
        ];
        fumen.pages[0].mirror = true;
        fumen.add_page();
        fumen.pages.push(Page::default());
        assert_eq!(fumen.encode(), "v115@bhwhglQpAtwwg0Q4A8LeAQLvhAAAAdhAAwDgHQLAPwSgWQaJeAAA");
        // assert_eq!(
        //     Fumen::decode("v115@bhwhglQpAtwwg0Q4A8LeAQLvhAAAAdhAAwDgHQLAPwSgWQaJeAAA"),
        //     Some(fumen)
        // );
    }

    #[test]
    fn comment() {
        let mut fumen = Fumen::default();
        fumen.pages[0].comment = Some("Hello World!".to_owned());
        assert_eq!(fumen.encode(), "v115@vhAAgWQAIoMDEvoo2AXXaDEkoA6A");
        // assert_eq!(Fumen::decode("v115@vhAAgWQAIoMDEvoo2AXXaDEkoA6A"), Some(fumen));
    }

    #[test]
    fn comment_unicode() {
        let mut fumen = Fumen::default();
        fumen.pages[0].comment = Some("こんにちは世界".to_owned());
        assert_eq!(
            fumen.encode(), "v115@vhAAgWqAlvs2A1sDfEToABBlvs2AWDEfET4J6Alvs2AWJEfE0H3KBlvtHB00AAA"
        );
        // assert_eq!(Fumen::decode(
        //    "v115@vhAAgWqAlvs2A1sDfEToABBlvs2AWDEfET4J6Alvs2AWJEfE0H3KBlvtHB00AAA"
        // ), Some(fumen));
    }

    #[test]
    fn comment_surrogate_pair() {
        let mut fumen = Fumen::default();
        fumen.pages[0].comment = Some("🂡🆛🏍😵".to_owned());
        assert_eq!(
            fumen.encode(),
            "v115@vhAAgWwAl/SSBzEEfEEFj6Al/SSBzEEfEkGpzBl/SSBzEEfEkpv6Bl/SSBTGEfEEojHB"
        );
        // assert_eq!(Fumen::decode(
        //    "v115@vhAAgWwAl/SSBzEEfEEFj6Al/SSBzEEfEkGpzBl/SSBzEEfEkpv6Bl/SSBTGEfEEojHB"
        // ), Some(fumen));
    }
}