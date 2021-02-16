use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use loterra_staking::msg::{
    ConfigResponse, GetAllHoldersResponse, GetHolderResponse, HandleMsg, InitMsg, QueryMsg,
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema_with_title(
        &mut schema_for!(GetHolderResponse),
        &out_dir,
        "GetHolderResponse",
    );
    export_schema_with_title(
        &mut schema_for!(GetAllHoldersResponse),
        &out_dir,
        "GetAllHoldersResponse",
    );
}