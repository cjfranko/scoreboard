use bytes::{BufMut, BytesMut, Bytes};
use std::io::{self, Error, ErrorKind};

/// Packet format for Ethernet communication with CPower control card
#[derive(Debug, Clone)]
pub struct EthernetPacket {
    pub network_data_length: u32,
    pub reserved: u16,
    pub packet_type: u8,
    pub card_type: u8,
    pub card_id: u8,
    pub command_data: Vec<u8>,
}

impl EthernetPacket {
    /// Create a new packet for sending to the scoreboard
    pub fn new(card_id: u8, command_data: Vec<u8>) -> Self {
        let data_length = 4 + command_data.len() as u32; // packet_type + card_type + card_id + command_data
        
        Self {
            network_data_length: data_length,
            reserved: 0,
            packet_type: 0x68, // Packet sent to control card
            card_type: 0x32,   // Constant code
            card_id,
            command_data,
        }
    }

    /// Encode the packet into bytes for transmission
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(8 + self.command_data.len());
        
        // ID Code (4 bytes) - always 0xffffffff for packets
        buf.put_u32(0xffffffff);
        
        // Network data length (2 bytes, little endian)
        buf.put_u16_le(self.network_data_length as u16);
        
        // Reserved (2 bytes)
        buf.put_u16_le(self.reserved);
        
        // Packet type (1 byte)
        buf.put_u8(self.packet_type);
        
        // Card type (1 byte)
        buf.put_u8(self.card_type);
        
        // Card ID (1 byte)
        buf.put_u8(self.card_id);
        
        // Command data
        buf.put_slice(&self.command_data);
        
        buf.freeze()
    }

    /// Decode a packet from received bytes
    pub fn decode(data: &[u8]) -> io::Result<Self> {
        if data.len() < 11 {
            return Err(Error::new(ErrorKind::InvalidData, "Packet too short"));
        }

        // Skip ID Code (4 bytes)
        let network_data_length = u16::from_le_bytes([data[4], data[5]]) as u32;
        let reserved = u16::from_le_bytes([data[6], data[7]]);
        let packet_type = data[8];
        let card_type = data[9];
        let card_id = data[10];
        
        let command_data = if data.len() > 11 {
            data[11..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            network_data_length,
            reserved,
            packet_type,
            card_type,
            card_id,
            command_data,
        })
    }

    /// Check if this is a response packet
    pub fn is_response(&self) -> bool {
        self.packet_type == 0xe8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_encode_decode() {
        let command_data = vec![0x47, 0x01, 0x01]; // Time query command
        let packet = EthernetPacket::new(0x01, command_data.clone());
        
        let encoded = packet.encode();
        let decoded = EthernetPacket::decode(&encoded).unwrap();
        
        assert_eq!(decoded.card_id, 0x01);
        assert_eq!(decoded.command_data, command_data);
        assert_eq!(decoded.packet_type, 0x68);
        assert_eq!(decoded.card_type, 0x32);
    }
}