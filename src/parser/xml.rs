use crate::cfg::sturct::Models;
use crate::models::account_management::xml::parse as parse_account_management;
use crate::models::application::xml::parse as parse_application;
use crate::models::authentication::xml::parse as parse_authentication;
use crate::models::powershell::xml::parse as parse_powershell;
use crate::models::scheduled_task::xml::parse as parse_scheduled_task;
use crate::models::service_control::xml::parse as parse_service_control;
use crate::models::session::xml::parse as parse_session;
use crate::parser::definition::EventRecord;

pub trait XmlParser {
    fn parse(&self, xml: &str) -> Box<dyn EventRecord + Send>;
}

impl XmlParser for Models {
    fn parse(&self, xml: &str) -> Box<dyn EventRecord + Send> {
        match self {
            Models::Application => parse_application(xml),
            Models::Authentication => parse_authentication(xml),
            Models::Session => parse_session(xml),
            Models::AccountManagement => parse_account_management(xml),
            Models::PowerShell => parse_powershell(xml),
            Models::ScheduledTask => parse_scheduled_task(xml),
            Models::ServiceControl => parse_service_control(xml),
        }
    }
}
