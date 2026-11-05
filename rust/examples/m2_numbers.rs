//! Completion M2: numeric policies and a standalone held/keyboard stepper.
mod common;
use scars_window::ui::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = State::new(9_007_199_254_740_993i64);
    let decimal = State::new(std::f64::consts::PI);
    let copies = State::new(3i64);
    let submitted = State::new(0);
    let cancelled = State::new(0);
    let exact = Numeric::new(count.binding(), 0, i64::MAX, 10);
    let precision = Numeric::new(decimal.binding(), -100., 100., 0.125)
        .significant_digits(6)
        .commit_on_blur(false)
        .on_submit(submitted.action(|v| v.update(|v| *v += 1)))
        .on_cancel(cancelled.action(|v| v.update(|v| *v += 1)));
    let step = Numeric::new(copies.binding(), 0, 10, 1);
    common::run("SCARS · Numbers", (720, 650), move || {
        view! {
            column(padding:28,gap:20){
                label("Numbers that keep their meaning",label_style:LabelStyle::Heading,wrap:true);
                panel(padding:18,gap:12){
                    numeric_field("Exact integer",&exact);
                    label(format!("Committed: {}",count.get()),label_style:LabelStyle::Secondary);
                    label("Blur or Enter applies a valid draft. Invalid and incomplete edits remain visible.",label_style:LabelStyle::Micro,wrap:true);
                }
                panel(padding:18,gap:12){
                    numeric_field("Decimal · six significant digits",&precision);
                    label(format!("Actual: {} · {:?}",decimal.get(),precision.validation().status),label_style:LabelStyle::Secondary,wrap:true);
                    label(format!("Enter applies · Escape reverts · {} submissions / {} cancellations",submitted.get(),cancelled.get()),label_style:LabelStyle::Micro,wrap:true);
                }
                row(gap:18,align:Align::Center){label(format!("Copies: {}",copies.get()));stepper("Copies",&step);}
                label("Hold +/− to repeat, or Tab to the stepper and use arrow keys. Repeats stop at bounds and on focus loss.",wrap:true,label_style:LabelStyle::Secondary);
            }
        }
    })
}
