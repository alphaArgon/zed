use gpui::Context;
use project::TaskSourceKind;
use remote::ConnectionState;
use task::{ResolvedTask, TaskContext, TaskTemplate};

use crate::Workspace;

pub fn schedule_task(
    workspace: &mut Workspace,
    task_source_kind: TaskSourceKind,
    task_to_resolve: &TaskTemplate,
    task_cx: &TaskContext,
    omit_history: bool,
    cx: &mut Context<Workspace>,
) {
    match workspace.project.read(cx).ssh_connection_state(cx) {
        None | Some(ConnectionState::Connected) => {}
        Some(
            ConnectionState::Connecting
            | ConnectionState::Disconnected
            | ConnectionState::HeartbeatMissed
            | ConnectionState::Reconnecting,
        ) => {
            log::warn!("Cannot schedule tasks when disconnected from a remote host");
            return;
        }
    }

    if let Some(spawn_in_terminal) =
        task_to_resolve.resolve_task(&task_source_kind.to_id_base(), task_cx)
    {
        schedule_resolved_task(
            workspace,
            task_source_kind,
            spawn_in_terminal,
            omit_history,
            cx,
        );
    }
}

pub fn schedule_resolved_task(
    workspace: &mut Workspace,
    task_source_kind: TaskSourceKind,
    mut resolved_task: ResolvedTask,
    omit_history: bool,
    cx: &mut Context<Workspace>,
) {
    let debug_config = resolved_task.resolved_debug_adapter_config();

    if let Some(spawn_in_terminal) = resolved_task.resolved.take() {
        if let Some(debug_config) = debug_config {
            workspace
                .debug_task_queue
                .insert(resolved_task.id.clone(), debug_config);
        }

        if !omit_history {
            resolved_task.resolved = Some(spawn_in_terminal.clone());
            workspace.project().update(cx, |project, cx| {
                if let Some(task_inventory) =
                    project.task_store().read(cx).task_inventory().cloned()
                {
                    task_inventory.update(cx, |inventory, _| {
                        inventory.task_scheduled(task_source_kind, resolved_task);
                    })
                }
            });
        }

        cx.emit(crate::Event::SpawnTask {
            action: Box::new(spawn_in_terminal),
        });
    }
}
