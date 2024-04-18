pub struct KernelContainer {
    pub platform: ocl::Platform,
    pub device: ocl::Device,
    pub context: ocl::Context,
    pub program: ocl::Program,
    pub queue: ocl::Queue
}

impl KernelContainer {
    pub fn new(source: &str) -> Self {
        let platform = ocl::Platform::default();
        let device = ocl::Device::first(platform).unwrap();
        let context = ocl::Context::builder()
            .platform(platform)
            .devices(device.clone())
            .build().unwrap();
        let program = ocl::Program::builder()
            .devices(device)
            .src(source)
            .build(&context).unwrap();
        let queue = ocl::Queue::new(&context, device, None).unwrap();
        return Self {
            platform,
            device,
            context,
            program,
            queue
        }
    }
}
