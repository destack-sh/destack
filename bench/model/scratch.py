from bench.model.base import load_model

if __name__ == "__main__":
    record = {
        "text": "Veritatis veritatis unde quo ut hic est et."
        "Rerum repellendus ut fuga dolorem vel. Sint eos nostrum porro."
        "Enim ut nihil molestias ea. Unde quia tempora explicabo consequatur ut corrupti."
    }
    # hf_model = HuggingFaceModel(
    #     model_name="nlptown/bert-base-multilingual-uncased-sentiment",
    # )
    # print(hf_model.forward(record))

    spacy_model = load_model(
        "bench.spacy.bundled",
        storage_uri=None,
        arguments={"model_name": "en_core_web_sm"},
        model_spec=None,
    )
    print(spacy_model.predict(record))

    # openai_model = OpenAIModel(
    #     api_key=get_from_env("OPENAI_API_KEY"),
    #     engine_id=OpenAIModel.Engine.CodexCushman,
    # )
    # print(openai_model.forward(record))
