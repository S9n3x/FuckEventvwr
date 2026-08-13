#[cfg(test)]
mod tests {
    use crate::models::account_management::xml::parse as parse_account_management;
    use crate::models::authentication::xml::parse as parse_authentication;
    use crate::models::powershell::xml::parse as parse_powershell;
    use crate::models::scheduled_task::xml::parse as parse_scheduled_task;
    use crate::models::service_control::xml::parse as parse_service_control;
    use crate::models::session::xml::parse as parse_session;

    const EXPECTED_TIME: &str = "2026-08-13T10:00:00.1234567Z";
    const XML: &str = concat!(
        "<Event><System><EventID>1000</EventID>",
        "<TimeCreated SystemTime=\"2026-08-13T10:00:00.1234567Z\"/>",
        "</System><EventData/></Event>"
    );

    #[test]
    fn all_existing_models_parse_self_closing_time_created() {
        let records = [
            parse_authentication(XML),
            parse_session(XML),
            parse_account_management(XML),
            parse_service_control(XML),
            parse_scheduled_task(XML),
            parse_powershell(XML),
        ];

        for record in records {
            assert_eq!(
                record.time(),
                EXPECTED_TIME,
                "{} 时间解析失败",
                record.type_name()
            );
        }
    }
}
