use std::sync::Arc;
use std::time::Duration;

use crate::driver::*;
use crate::Care;

use shrine_storage::{Gift, Mode, store::Namespace};

use tokio::sync::mpsc;
use async_trait::async_trait;


pub struct OneShotItem {
    pub path: String,
    pub item: Arc<dyn IntoTale>,
}

pub struct OneShotDriver {
    id: DriverID,
    items: Vec<OneShotItem>,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
    wait: Option<Duration>
}

impl OneShotDriver {
    fn set_wait(&mut self, d: Duration) {
        self.wait = Some(d);
    }

    pub fn new_lazy(time: Duration, id: DriverID, ns: Arc<Namespace>, items: impl Into<Vec<OneShotItem>>) -> Self {
        let mut res = Self::new(id, ns, items);
        res.set_wait(time);
        return res;
    }

    pub fn new(id: DriverID, ns: Arc<Namespace>, items: impl Into<Vec<OneShotItem>>) -> Self {
        Self {
            id,
            wait: None,
            items: items.into(),
            namespace: ns,
            tx: None
        }
    }

    pub fn append(&mut self, items: impl Iterator<Item = OneShotItem>) {
        for i in items {
            self.items.push(i);
        }
    }


    async fn begin(&mut self) {
        if let Some(wait) = self.wait {
            tokio::time::sleep(wait).await;
        }
        let Some(tx) = self.tx.as_ref() else {
            //
            panic!("fuck it");
        };
        let mut id = 0;
        for it in self.items.drain(..) {
            let note = DriverNote {
                path: it.path,
                mode: Mode::Add,
                slots: it.item
            };
            let card = DriverCard {
                id,
                note
            };

            let _ = tx.send(vec![card]).await;
            id += 1;
        }

    }

}



#[async_trait]
impl ShrineDriver for OneShotDriver {
    fn id(&self) -> DriverID {
        self.id
    }

    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id("/").unwrap().raw(),
            care: Care::X
        }
    }

    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }



    async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.begin().await;

        Ok(())
    }

    async fn on_dirty(&mut self, _dirty: &[Gift]) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }
}

