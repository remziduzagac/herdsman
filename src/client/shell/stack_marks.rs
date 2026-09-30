//! Stack membership for the navigator. Stacks are server layout, so the
//! navigator asks the endpoint once when it opens and marks stacked rows.
//! Endpoints without `pane.stacks` simply get no marks.

use std::collections::HashMap;

use super::{
    ClientEndpointId, ClientShellEndpointError, ClientShellInput, ClientShellOverlay,
    ClientShellState, PendingEndpointKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StackMark {
    position: usize,
    len: usize,
    visible: bool,
}

/// Marks for one endpoint's stacked panes, keyed by public pane id.
#[derive(Debug, Default)]
pub(super) struct NavigatorStacks {
    endpoint_id: Option<ClientEndpointId>,
    marks: HashMap<String, StackMark>,
}

impl NavigatorStacks {
    /// `label` with the pane's place in its stack, and whether it is hidden,
    /// so typing "stacked" or "hidden" filters to them.
    pub(super) fn label(
        &self,
        endpoint_id: &ClientEndpointId,
        pane_id: &str,
        label: String,
    ) -> String {
        if self.endpoint_id.as_ref() != Some(endpoint_id) {
            return label;
        }
        match self.marks.get(pane_id) {
            Some(mark) if mark.visible => {
                format!("{label} · stacked {}/{}", mark.position, mark.len)
            }
            Some(mark) => format!("{label} · stacked {}/{} · hidden", mark.position, mark.len),
            None => label,
        }
    }
}

impl ClientShellState {
    /// Ask the active endpoint for its stacks, quietly: no notice when the
    /// endpoint is offline or does not advertise `pane.stacks`.
    pub(super) fn request_navigator_stacks(&mut self, outcome: &mut ClientShellInput) {
        let method = crate::api::schema::Method::PaneStacks(Default::default());
        if !self.endpoint_is_online(&self.active_endpoint_id)
            || !self.supports_endpoint_method(&method)
        {
            return;
        }
        let endpoint_id = self.active_endpoint_id.clone();
        self.push_endpoint_method_with_kind(
            method,
            PendingEndpointKind::NavigatorStacks { endpoint_id },
            outcome,
        );
    }

    pub(super) fn receive_navigator_stacks(
        &mut self,
        endpoint_id: ClientEndpointId,
        result: Result<crate::api::schema::ResponseResult, ClientShellEndpointError>,
    ) -> bool {
        let Ok(crate::api::schema::ResponseResult::PaneStacks { stacks }) = result else {
            return false;
        };
        let Some(ClientShellOverlay::Navigator(navigator)) = self.overlay.as_mut() else {
            return false;
        };
        let mut marks = HashMap::new();
        for stack in stacks {
            let len = stack.pane_ids.len();
            for (index, pane_id) in stack.pane_ids.into_iter().enumerate() {
                let visible = pane_id == stack.visible_pane_id;
                marks.insert(
                    pane_id,
                    StackMark {
                        position: index + 1,
                        len,
                        visible,
                    },
                );
            }
        }
        navigator.stacks = NavigatorStacks {
            endpoint_id: Some(endpoint_id),
            marks,
        };
        true
    }
}
