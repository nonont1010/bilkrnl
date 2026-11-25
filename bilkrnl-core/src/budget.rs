//! Resource budgets for enclave execution.

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_cycles: u64,
    pub max_out_bytes: u32,
    pub max_mem_pages: u16,
}

impl ResourceBudget {
    pub const fn new(max_cycles: u64, max_out_bytes: u32, max_mem_pages: u16) -> Self {
        Self {
            max_cycles,
            max_out_bytes,
            max_mem_pages,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    CyclesExceeded { used: u64, max: u64 },
    OutBytesExceeded { used: u32, max: u32 },
    MemPagesExceeded { used: u16, max: u16 },
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BudgetError::CyclesExceeded { used, max } => {
                write!(f, "cycles exceeded: {} > {}", used, max)
            }
            BudgetError::OutBytesExceeded { used, max } => {
                write!(f, "out bytes exceeded: {} > {}", used, max)
            }
            BudgetError::MemPagesExceeded { used, max } => {
                write!(f, "mem pages exceeded: {} > {}", used, max)
            }
        }
    }
}

/// Mutable counters checked against a [`ResourceBudget`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BudgetAccount {
    pub cycles: u64,
    pub out_bytes: u32,
    pub mem_high_water_pages: u16,
}

impl BudgetAccount {
    pub fn charge_cycle(&mut self, budget: &ResourceBudget) -> Result<(), BudgetError> {
        let next = self.cycles.saturating_add(1);
        if next > budget.max_cycles {
            self.cycles = next;
            return Err(BudgetError::CyclesExceeded {
                used: next,
                max: budget.max_cycles,
            });
        }
        self.cycles = next;
        Ok(())
    }

    pub fn charge_out(&mut self, n: u32, budget: &ResourceBudget) -> Result<(), BudgetError> {
        let next = self.out_bytes.saturating_add(n);
        if next > budget.max_out_bytes {
            return Err(BudgetError::OutBytesExceeded {
                used: next,
                max: budget.max_out_bytes,
            });
        }
        self.out_bytes = next;
        Ok(())
    }

    pub fn note_addr(&mut self, addr: u32, budget: &ResourceBudget) -> Result<(), BudgetError> {
        let page = (addr / crate::isa::PAGE_SIZE as u32) as u16;
        let pages_touched = page.saturating_add(1);
        if pages_touched > budget.max_mem_pages {
            return Err(BudgetError::MemPagesExceeded {
                used: pages_touched,
                max: budget.max_mem_pages,
            });
        }
        if pages_touched > self.mem_high_water_pages {
            self.mem_high_water_pages = pages_touched;
        }
        Ok(())
    }
}
