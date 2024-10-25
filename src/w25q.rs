use defmt::info;
use embassy_stm32::{
    mode::Blocking,
    ospi::{AddressSize, DummyCycles, Instance, Ospi, OspiWidth, TransferConfig},
};

const MEMORY_PAGE_SIZE: usize = 256;

const CMD_READ: u8 = 0x03;
const CMD_HS_READ: u8 = 0x0B;
const CMD_QUAD_READ: u8 = 0x6B;

const CMD_WRITE_PG: u8 = 0xF2;
const CMD_QUAD_WRITE_PG: u8 = 0x32;

const CMD_READ_ID: u8 = 0x9F;
const CMD_READ_UUID: u8 = 0x4B;

const CMD_ENABLE_RESET: u8 = 0x66;
const CMD_RESET: u8 = 0x99;

const CMD_WRITE_ENABLE: u8 = 0x06;
const CMD_WRITE_DISABLE: u8 = 0x04;

const CMD_CHIP_ERASE: u8 = 0xC7;
const CMD_SECTOR_ERASE: u8 = 0x20;
const CMD_BLOCK_ERASE_32K: u8 = 0x52;
const CMD_BLOCK_ERASE_64K: u8 = 0xD8;

const CMD_READ_SR: u8 = 0x05;
const CMD_READ_CR: u8 = 0x35;

const CMD_WRITE_SR: u8 = 0x01;
const CMD_WRITE_CR: u8 = 0x31;

/// Implementation of access to flash chip.
/// Chip commands are hardcoded as it depends on used chip.
/// This implementation is using chip GD25Q64C from Giga Device
pub struct FlashMemory<I: Instance> {
    ospi: Ospi<'static, I, Blocking>,
}

impl<I: Instance> FlashMemory<I> {
    pub async fn new(ospi: Ospi<'static, I, Blocking>) -> Self {
        let mut memory = Self { ospi };

        memory.reset_memory().await;
        memory.enable_quad();
        memory
    }

    async fn qpi_mode(&mut self) {
        // Enter qpi mode
        self.exec_command(0x38).await;
        
        // Set read param
        let transaction = TransferConfig {
            iwidth: OspiWidth::QUAD,
            dwidth: OspiWidth::QUAD,
            instruction: Some(0xC0),
            ..Default::default()
        };
        self.enable_write().await;
        self.ospi.blocking_write(&[0x30_u8], transaction).unwrap();
        self.wait_write_finish();
    }

    pub async fn enable_mm(&mut self) {
        self.qpi_mode().await;

        let read_config = TransferConfig {
            iwidth: OspiWidth::QUAD,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::QUAD,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::QUAD,
            instruction: Some(0x0B), // Fast read in QPI mode
            dummy: DummyCycles::_8,
            ..Default::default()
        };

        let write_config = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::SING,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::QUAD,
            instruction: Some(0x32), // Write config
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.ospi.enable_memory_mapped_mode(read_config, write_config).unwrap();
    }

    fn enable_quad(&mut self) {
        let cr = self.read_cr();
        // info!("Read cr: {:x}", cr);
        self.write_cr(cr | 0x02);
        // info!("Read cr after writing: {:x}", cr);
    }

    pub fn disable_quad(&mut self) {
        let cr = self.read_cr();
        self.write_cr(cr & (!(0x02)));
    }

    async fn exec_command_4(&mut self, cmd: u8) {
        let transaction = TransferConfig {
            iwidth: OspiWidth::QUAD,
            adwidth: OspiWidth::NONE,
            // adsize: AddressSize::_24bit,
            dwidth: OspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.ospi.command(&transaction).await.unwrap();
    }

    async fn exec_command(&mut self, cmd: u8) {
        let transaction = TransferConfig {
            iwidth: OspiWidth::SING,
            adwidth: OspiWidth::NONE,
            // adsize: AddressSize::_24bit,
            dwidth: OspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        // info!("Excuting command: {:x}", transaction.instruction);
        self.ospi.command(&transaction).await.unwrap();
    }

    pub async fn reset_memory(&mut self) {
        self.exec_command_4(CMD_ENABLE_RESET).await;
        self.exec_command_4(CMD_RESET).await;
        self.exec_command(CMD_ENABLE_RESET).await;
        self.exec_command(CMD_RESET).await;
        self.wait_write_finish();
    }

    pub async fn enable_write(&mut self) {
        self.exec_command(CMD_WRITE_ENABLE).await;
    }

    pub fn read_id(&mut self) -> [u8; 3] {
        let mut buffer = [0; 3];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::NONE,
            // adsize: AddressSize::_24bit,
            dwidth: OspiWidth::SING,
            instruction: Some(CMD_READ_ID as u32),
            ..Default::default()
        };
        // info!("Reading id: 0x{:X}", transaction.instruction);
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    pub fn read_id_4(&mut self) -> [u8; 3] {
        let mut buffer = [0; 3];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::NONE,
            dwidth: OspiWidth::QUAD,
            instruction: Some(CMD_READ_ID as u32),
            ..Default::default()
        };
        info!("Reading id: 0x{:X}", transaction.instruction);
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    pub fn read_uuid(&mut self) -> [u8; 16] {
        let mut buffer = [0; 16];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            adwidth: OspiWidth::SING,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::SING,
            instruction: Some(CMD_READ_UUID as u32),
            address: Some(0),
            dummy: DummyCycles::_8,
            ..Default::default()
        };
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    pub fn read_memory(&mut self, addr: u32, buffer: &mut [u8], use_dma: bool) {
        let transaction = TransferConfig {
            iwidth: OspiWidth::SING,
            adwidth: OspiWidth::SING,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::QUAD,
            instruction: Some(CMD_QUAD_READ as u32),
            address: Some(addr),
            dummy: DummyCycles::_8,
            ..Default::default()
        };
        if use_dma {
            self.ospi.blocking_read(buffer, transaction).unwrap();
        } else {
            self.ospi.blocking_read(buffer, transaction).unwrap();
        }
    }

    fn wait_write_finish(&mut self) {
        while (self.read_sr() & 0x01) != 0 {}
    }

    async fn perform_erase(&mut self, addr: u32, cmd: u8) {
        let transaction = TransferConfig {
            iwidth: OspiWidth::SING,
            adwidth: OspiWidth::SING,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write().await;
        self.ospi.command(&transaction).await.unwrap();
        self.wait_write_finish();
    }

    pub async fn erase_sector(&mut self, addr: u32) {
        self.perform_erase(addr, CMD_SECTOR_ERASE).await;
    }

    pub async fn erase_block_32k(&mut self, addr: u32) {
        self.perform_erase(addr, CMD_BLOCK_ERASE_32K).await;
    }

    pub async fn erase_block_64k(&mut self, addr: u32) {
        self.perform_erase(addr, CMD_BLOCK_ERASE_64K).await;
    }

    pub async fn erase_chip(&mut self) {
        self.exec_command(CMD_CHIP_ERASE).await;
    }

    async fn write_page(&mut self, addr: u32, buffer: &[u8], len: usize, use_dma: bool) {
        assert!(
            (len as u32 + (addr & 0x000000ff)) <= MEMORY_PAGE_SIZE as u32,
            "write_page(): page write length exceeds page boundary (len = {}, addr = {:X}",
            len,
            addr
        );

        let transaction = TransferConfig {
            iwidth: OspiWidth::SING,
            adsize: AddressSize::_24bit,
            adwidth: OspiWidth::SING,
            dwidth: OspiWidth::QUAD,
            instruction: Some(CMD_QUAD_WRITE_PG as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write().await;
        if use_dma {
            self.ospi.blocking_write(buffer, transaction).unwrap();
        } else {
            self.ospi.blocking_write(buffer, transaction).unwrap();
        }
        self.wait_write_finish();
    }

    pub async fn write_memory(&mut self, addr: u32, buffer: &[u8], use_dma: bool) {
        let mut left = buffer.len();
        let mut place = addr;
        let mut chunk_start = 0;

        while left > 0 {
            let max_chunk_size = MEMORY_PAGE_SIZE - (place & 0x000000ff) as usize;
            let chunk_size = if left >= max_chunk_size {
                max_chunk_size
            } else {
                left
            };
            let chunk = &buffer[chunk_start..(chunk_start + chunk_size)];
            self.write_page(place, chunk, chunk_size, use_dma).await;
            place += chunk_size as u32;
            left -= chunk_size;
            chunk_start += chunk_size;
        }
    }

    fn read_register(&mut self, cmd: u8) -> u8 {
        let mut buffer = [0; 1];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::NONE,
            adsize: AddressSize::_24bit,
            dwidth: OspiWidth::SING,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        // info!("Read w25q64 register: 0x{:x}", buffer[0]);
        buffer[0]
    }

    fn write_register(&mut self, cmd: u8, value: u8) {
        let buffer = [value; 1];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            instruction: Some(cmd as u32),
            adsize: AddressSize::_24bit,
            adwidth: OspiWidth::NONE,
            dwidth: OspiWidth::SING,
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.ospi.blocking_write(&buffer, transaction).unwrap();
    }

    pub fn read_sr(&mut self) -> u8 {
        self.read_register(CMD_READ_SR)
    }

    pub fn read_cr(&mut self) -> u8 {
        self.read_register(CMD_READ_CR)
    }

    pub fn write_sr(&mut self, value: u8) {
        self.write_register(CMD_WRITE_SR, value);
    }

    pub fn write_cr(&mut self, value: u8) {
        self.write_register(CMD_WRITE_CR, value);
    }
}
