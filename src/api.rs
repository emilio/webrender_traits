use display_list::DisplayListBuilder;
use euclid::Point2D;
use stacking_context::StackingContext;
use std::cell::Cell;
use std::sync::mpsc::{self, Sender};
use types::{ColorF, DisplayListId, Epoch, FontKey, StackingContextId};
use types::{ImageKey, ImageFormat, NativeFontHandle, PipelineId};

#[derive(Clone, Copy, Debug)]
pub struct IdNamespace(pub u32);

#[derive(Clone, Copy, Debug)]
pub struct ResourceId(pub u32);

pub enum ApiMsg {
    AddRawFont(FontKey, Vec<u8>),
    AddNativeFont(FontKey, NativeFontHandle),
    AddImage(ImageKey, u32, u32, ImageFormat, Vec<u8>),
    UpdateImage(ImageKey, u32, u32, ImageFormat, Vec<u8>),
    AddDisplayList(DisplayListId, PipelineId, Epoch, DisplayListBuilder),
    AddStackingContext(StackingContextId, PipelineId, Epoch, StackingContext),
    CloneApi(Sender<RenderApi>),
    SetRootStackingContext(StackingContextId, ColorF, Epoch, PipelineId),
    SetRootPipeline(PipelineId),
    Scroll(Point2D<f32>),
    TranslatePointToLayerSpace(Point2D<f32>, Sender<Point2D<f32>>),
}

pub struct RenderApi {
    pub tx: Sender<ApiMsg>,
    pub id_namespace: IdNamespace,
    pub next_id: Cell<ResourceId>,
}

impl RenderApi {
    pub fn new(api_tx: Sender<ApiMsg>) -> RenderApi {
        RenderApi {
            tx: api_tx,
            id_namespace: IdNamespace(0),   // special case
            next_id: Cell::new(ResourceId(0)),
        }
    }

    pub fn add_raw_font(&self, bytes: Vec<u8>) -> FontKey {
        let new_id = self.next_unique_id();
        let key = FontKey::new(new_id.0, new_id.1);
        let msg = ApiMsg::AddRawFont(key, bytes);
        self.tx.send(msg).unwrap();
        key
    }

    pub fn add_native_font(&self, native_font_handle: NativeFontHandle) -> FontKey {
        let new_id = self.next_unique_id();
        let key = FontKey::new(new_id.0, new_id.1);
        let msg = ApiMsg::AddNativeFont(key, native_font_handle);
        self.tx.send(msg).unwrap();
        key
    }

    pub fn alloc_image(&self) -> ImageKey {
        let new_id = self.next_unique_id();
        ImageKey::new(new_id.0, new_id.1)
    }

    pub fn add_image(&self,
                     width: u32,
                     height: u32,
                     format: ImageFormat,
                     bytes: Vec<u8>) -> ImageKey {
        let new_id = self.next_unique_id();
        let key = ImageKey::new(new_id.0, new_id.1);
        let msg = ApiMsg::AddImage(key, width, height, format, bytes);
        self.tx.send(msg).unwrap();
        key
    }

    // TODO: Support changing dimensions (and format) during image update?
    pub fn update_image(&self,
                        key: ImageKey,
                        width: u32,
                        height: u32,
                        format: ImageFormat,
                        bytes: Vec<u8>) {
        let msg = ApiMsg::UpdateImage(key, width, height, format, bytes);
        self.tx.send(msg).unwrap();
    }

    pub fn add_display_list(&self,
                            display_list: DisplayListBuilder,
                            stacking_context: &mut StackingContext,
                            pipeline_id: PipelineId,
                            epoch: Epoch) -> Option<DisplayListId> {
        //TODO! debug_assert!(display_list.item_count() > 0, "Avoid adding empty lists!");
        let new_id = self.next_unique_id();
        let id = DisplayListId(new_id.0, new_id.1);
        stacking_context.has_stacking_contexts = stacking_context.has_stacking_contexts ||
                                                 display_list.has_stacking_contexts;
        let msg = ApiMsg::AddDisplayList(id, pipeline_id, epoch, display_list);
        self.tx.send(msg).unwrap();
        stacking_context.display_lists.push(id);

        Some(id)
    }

    pub fn add_stacking_context(&self,
                                stacking_context: StackingContext,
                                pipeline_id: PipelineId,
                                epoch: Epoch) -> StackingContextId {
        let new_id = self.next_unique_id();
        let id = StackingContextId(new_id.0, new_id.1);
        let msg = ApiMsg::AddStackingContext(id, pipeline_id, epoch, stacking_context);
        self.tx.send(msg).unwrap();
        id
    }

    pub fn set_root_pipeline(&self, pipeline_id: PipelineId) {
        let msg = ApiMsg::SetRootPipeline(pipeline_id);
        self.tx.send(msg).unwrap();
    }

    pub fn set_root_stacking_context(&self,
                                     stacking_context_id: StackingContextId,
                                     background_color: ColorF,
                                     epoch: Epoch,
                                     pipeline_id: PipelineId) {
        let msg = ApiMsg::SetRootStackingContext(stacking_context_id,
                                                 background_color,
                                                 epoch,
                                                 pipeline_id);
        self.tx.send(msg).unwrap();
    }

    pub fn scroll(&self, delta: Point2D<f32>) {
        let msg = ApiMsg::Scroll(delta);
        self.tx.send(msg).unwrap();
    }

    pub fn translate_point_to_layer_space(&self, point: &Point2D<f32>) -> Point2D<f32> {
        let (tx, rx) = mpsc::channel();
        let msg = ApiMsg::TranslatePointToLayerSpace(*point, tx);
        self.tx.send(msg).unwrap();
        rx.recv().unwrap()
    }

    pub fn clone_api(&self) -> RenderApi {
        let (tx, rx) = mpsc::channel();
        let msg = ApiMsg::CloneApi(tx);
        self.tx.send(msg).unwrap();
        rx.recv().unwrap()
    }

    fn next_unique_id(&self) -> (u32, u32) {
        let IdNamespace(namespace) = self.id_namespace;
        let ResourceId(id) = self.next_id.get();
        self.next_id.set(ResourceId(id + 1));
        (namespace, id)
    }
}
