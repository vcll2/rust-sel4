use core::future::Future;

use smoltcp::iface::Config;
use smoltcp::phy::{Device, Medium};
use smoltcp::time::{Duration, Instant};
use smoltcp::wire::HardwareAddress;

use sel4_async_network::SharedNetwork;
use sel4_async_single_threaded_executor::{LocalPool, LocalSpawner};

use tests_capdl_http_server_components_test_sp804_driver::Driver as TimerDriver;

use crate::net::Net;

const TIMER_IRQ_BADGE: sel4::Badge = 1 << 0;
const VIRTIO_NET_IRQ_BADGE: sel4::Badge = 1 << 1;

pub struct Ctx {
    net: Net,
    net_irq_handler: sel4::IRQHandler,
    timer: TimerDriver,
    timer_irq_handler: sel4::IRQHandler,
    shared_network: SharedNetwork,
}

impl Ctx {
    pub fn new(
        timer: TimerDriver,
        timer_irq_handler: sel4::IRQHandler,
        mut net: Net,
        net_irq_handler: sel4::IRQHandler,
    ) -> Self {
        let mut config = Config::new();
        config.random_seed = 0;
        if net.capabilities().medium == Medium::Ethernet {
            config.hardware_addr = Some(HardwareAddress::Ethernet(net.mac_address()));
        }

        let shared_network = SharedNetwork::new(config, &mut net);

        Self {
            timer,
            timer_irq_handler,
            net,
            net_irq_handler,
            shared_network,
        }
    }

    pub fn now(&mut self) -> Instant {
        Instant::from_micros(i64::try_from(self.timer.now().as_micros()).unwrap())
    }

    pub fn set_timeout(&mut self, d: Duration) {
        self.timer
            .set_timeout(core::time::Duration::from_micros(d.micros()))
    }

    pub fn run<T: Future<Output = !>>(
        mut self,
        event_nfn: sel4::Notification,
        f: impl FnOnce(SharedNetwork, LocalSpawner) -> T,
    ) -> ! {
        self.net.device().borrow_mut().ack_interrupt();
        self.net_irq_handler.irq_handler_ack().unwrap();

        self.timer.handle_interrupt();
        self.timer_irq_handler.irq_handler_ack().unwrap();

        let mut local_pool = LocalPool::new();
        let spawner = local_pool.spawner();

        let fut = f(self.shared_network.clone(), spawner);
        futures::pin_mut!(fut);

        loop {
            loop {
                let _ = local_pool.run_until_stalled(fut.as_mut());
                let timestamp = self.now();
                if !self
                    .shared_network
                    .inner()
                    .borrow_mut()
                    .poll(timestamp, &mut self.net)
                {
                    break;
                }
            }

            let timestamp = self.now();
            let delay = self
                .shared_network
                .inner()
                .borrow_mut()
                .poll_delay(timestamp);
            if let Some(d) = delay {
                log::trace!("poll delay: {:?}", d);
                self.set_timeout(d);
            }

            let (_, badge) = event_nfn.wait();

            if badge & TIMER_IRQ_BADGE != 0 {
                log::trace!("timer interrupt at now={:?}", self.now());
                self.timer.handle_interrupt();
                self.timer_irq_handler.irq_handler_ack().unwrap();
            }

            if badge & VIRTIO_NET_IRQ_BADGE != 0 {
                log::trace!("net interrupt at now={:?}", self.now());
                self.net.device().borrow_mut().ack_interrupt();
                self.net_irq_handler.irq_handler_ack().unwrap();
            }
        }
    }
}