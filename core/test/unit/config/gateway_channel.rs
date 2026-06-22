    use super::*;

    #[test]
    fn catalog_lists_wecom_as_available() {
        let wecom = gateway_channel("wecom").unwrap();
        assert!(wecom.available);
    }

    #[test]
    fn catalog_lists_feishu_as_available() {
        let feishu = gateway_channel("feishu").unwrap();
        assert!(feishu.available);
    }

    #[test]
    fn catalog_lists_weixin_as_available() {
        let weixin = gateway_channel("weixin").unwrap();
        assert!(weixin.available);
    }

    #[test]
    fn planned_channels_are_not_available() {
        for id in ["dingtalk", "telegram", "slack"] {
            let ch = gateway_channel(id).unwrap();
            assert!(!ch.available);
        }
    }
