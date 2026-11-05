use super::model::Todos;
use scars_window::ui::*;
pub fn view(model: &Todos) -> View {
    let remaining = model.remaining();
    let total = model.items.get().len();
    let visible = model.visible();
    let editing = model.editing.get();
    view! {
        column(padding:32,gap:22){
            label("Things to do",font_size:38,bold:true,shrink:0);
            label("A little space for what comes next.",shrink:0);
            row(shrink:0,gap:12){
                text_field("What needs to be done?",bind:model.input,autofocus:true,grow:1,height:44,on_submit:model.action(Todos::add));
                button("Add task",height:44,on_click:model.action(Todos::add));
            }
            row(shrink:0){
                checkbox("Complete all",bind:model.all_done(),enabled:total>0,grow:1);
                segmented(bind:model.filter,options:vec![OptionItem::new("all","All"),OptionItem::new("active","Active"),OptionItem::new("completed","Completed")]);
            }
            panel(padding:16,grow:1){
                scroll(grow:1,gap:12){
                    for todo in visible {
                        row(key:todo.id,gap:10,align:Align::Center,height:48,shrink:0){
                            checkbox("",bind:model.done(todo.id),width:26,accessible_label:format!("Complete {}",todo.title));
                            if editing==Some(todo.id){
                                text_field("Task",bind:model.draft,grow:1,autofocus:true,on_submit:model.action(Todos::commit_edit),on_cancel:model.action(Todos::cancel_edit));
                                button("Save",on_click:model.action(Todos::commit_edit));
                                button("Cancel",on_click:model.action(Todos::cancel_edit));
                            }else{
                                label(todo.title.clone(),grow:1,wrap:true);
                                button("Edit",width:50,on_click:model.action(move|m|m.begin_edit(todo.id)),accessible_label:format!("Edit {}",todo.title));
                                button("×",width:32,on_click:model.action(move|m|m.remove(todo.id)),accessible_label:format!("Delete {}",todo.title));
                            }
                        }
                        separator();
                    }
                    if total==0{label("A fresh start",font_size:22,bold:true);label("Add your first task above. Enter adds it to the list.",wrap:true);}
                    else if model.visible().is_empty(){label("Nothing in this view. You’re all caught up.",wrap:true);}
                }
            }
            row(shrink:0,align:Align::Center){
                label(format!("{remaining} {} left",if remaining==1{"task"}else{"tasks"}),grow:1);
                button("Clear completed",enabled:remaining<total,on_click:model.action(Todos::clear_completed));
            }
            if let Some(error)=model.error.get(){label(error,wrap:true,foreground:0xffff8787,shrink:0);}
        }
    }
}
